// std
use std::{sync::Arc, time::Duration};

// poem
use poem::{
    endpoint::{BoxEndpoint, EndpointExt, PrometheusExporter},
    get, post, middleware::{AddDataEndpoint, OpenTelemetryTracing},
    Response, Route,
};

// OpenTelemetry
use opentelemetry::{global, trace::TracerProvider as _};
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{SdkTracerProvider},
    Resource,
};

// tracing
use tracing_subscriber::{layer::SubscriberExt, registry};

// local crate
use crate::{
    articles::{get_article_by_id, get_articles, post_article, ArticleList, ArticleStore},
    config::Config,
    exporter::Metrics,
    fault_inject::FaultInject,
    logging::Logger,
    status::up,
};

// handy alias
type DynHandler = BoxEndpoint<'static, Response>;

// 3) Struktura popisující jednu routu
struct RouteDef {
    path: &'static str,
    handler: DynHandler,
}

#[derive(Clone)]
pub struct AppState {
    pub store: ArticleStore,
    pub log: Arc<Logger>,
    pub metrics: Metrics,
    registry: prometheus::Registry,
}

impl AppState {
    pub fn build(config: &Config) -> Self {

        let store = ArticleStore::new(&ArticleList::new());
        let log = Arc::new(Logger::build(&config.log_output));
        let registry = prometheus::Registry::new();
        let metrics = Metrics::new(&registry);

        AppState { store, log, metrics, registry }
    }
}

pub async fn builder(config: &Config) -> AddDataEndpoint<Route, AppState> {
    
    let state = AppState::build(&config);
    let log = state.log.clone();
    let exporter = PrometheusExporter::new(state.registry.clone());
    let fault_inject = FaultInject::new()
        .with_error_rate(0.1)
        .with_delay(std::time::Duration::from_millis(50), std::time::Duration::from_millis(500))
        .with_timeout(std::time::Duration::from_secs(2))
        .with_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR);
    let tracer = init_tracer().tracer("simple-api-trace");

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());
    let subscriber = tracing_subscriber::Registry::default().with(otel_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Unable to set global subscriber.");

    let routes: Vec<RouteDef> = vec![
        RouteDef {
            path: "/api/v1/status",
            handler: get(up).boxed(),
        },
        RouteDef {
            path: "/api/v1/articles",
            handler: get(get_articles).boxed(),           // <– EndpointExt::boxed()
        },
        RouteDef {
            path: "/api/v1/article/:1",
            handler: get(get_article_by_id).boxed(),
        },
        RouteDef {
            path: "/api/v1/article",
            handler: post(post_article).boxed(),
        },
    ];

    let api =  routes
    .into_iter()
    .fold(Route::new(), |app, def| app.at(def.path, def.handler))
    .with(OpenTelemetryTracing::new(tracer))
    .with(fault_inject);

    let route=  Route::new()
        .nest("/", api)
        .nest("/metrics", exporter)
        .data(state);

    log.info("Application initialized".into()).await;
    log.info(format!("Starting server on {}:{}", config.addr, config.port)).await;

    route
}


fn init_tracer() -> SdkTracerProvider {
    global::set_text_map_propagator(TraceContextPropagator::new());
    let provider = SdkTracerProvider::builder()
        .with_resource(Resource::builder().with_service_name("simple-api").build())
        .with_batch_exporter(
            opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_protocol(Protocol::Grpc)
                .with_endpoint("http://localhost:9821")
                .with_timeout(Duration::from_secs(3))
                .build()
                .expect("Trace exporter should initialize."),
        )
        .build();

    provider
}