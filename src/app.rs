use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use poem::{ endpoint::{BoxEndpoint, EndpointExt, PrometheusExporter}, get, post, middleware::AddDataEndpoint, Route };
use poem::middleware::OpenTelemetryTracing;
use std::sync::Arc;

use crate::{articles::{ get_article_by_id, post_article }, config::Config, status::up};
use crate::articles::{ get_articles, ArticleStore, ArticleList };
use crate::logging::Logger;
use crate::exporter::Metrics;
use crate::fault_inject::FaultInject;


use opentelemetry::{global, trace::TracerProvider as _};
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::{SdkTracerProvider, Tracer}, Resource};
use opentelemetry_otlp::Protocol;
use std::time::Duration;

type DynHandler = BoxEndpoint<'static, poem::Response>;



// 3) Struktura popisující jednu routu
struct RouteDef {
    path: &'static str,
    handler: DynHandler,
}

#[derive(Clone)]
pub struct AppState {
    pub store: ArticleStore,
    pub log: Arc<Logger>,
    pub metrics: Metrics
}

impl AppState {
    pub fn build(config: &Config, metrics: Metrics) -> Self {

        let store = ArticleStore::new(&ArticleList::new());
        let log = Arc::new(Logger::build(&config.log_output));

        AppState { store, log, metrics }
    }
}

pub async fn builder(config: &Config) -> AddDataEndpoint<Route, AppState> {

    let registry = prometheus::Registry::new();
    let state = AppState::build(&config, Metrics::new(&registry));
    let log = state.log.clone();
    let fault_inject = FaultInject::new()
        .with_error_rate(0.1)
        .with_delay(std::time::Duration::from_millis(50), std::time::Duration::from_millis(100))
        .with_timeout(std::time::Duration::from_secs(2))
        .with_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR);

    let tracer_provider = init_tracer();
    let tracer = tracer_provider.tracer("simple-api-rs");

    let routes: Vec<RouteDef> = vec![
        RouteDef {
            path: "/api/v1/status",
            handler: get(up).boxed(),
     // <– EndpointExt::boxed()
        },
        RouteDef {
            path: "/api/v1/articles",
            handler: get(get_articles).boxed(),           // <– EndpointExt::boxed()
        },
        RouteDef {
            path: "/api/v1/article/:1",
            handler: get(get_article_by_id).boxed(),
        // <– EndpointExt::boxed()
        },
        RouteDef {
            path: "/api/v1/article",
            handler: post(post_article).boxed(),
                     // <– EndpointExt::boxed()
        },
    ];

    let api_only =  routes
    .into_iter()
    .fold(Route::new(), |app, def| app.at(def.path, def.handler))
    .with(OpenTelemetryTracing::new(tracer))
    .with(fault_inject);

    let route=  Route::new()
        .nest("/", api_only)
        .nest("/metrics", PrometheusExporter::new(registry.clone()))
        .data(state);

    log.info("Application initialized".into()).await;
    log.info(format!("Starting server on {}:{}", config.addr, config.port)).await;


    route
}


fn init_tracer() -> SdkTracerProvider {
    global::set_text_map_propagator(TraceContextPropagator::new());
    SdkTracerProvider::builder()
        .with_resource(Resource::builder().with_service_name("server2").build())
        .with_batch_exporter(
            opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_protocol(Protocol::Grpc)
                .with_endpoint("http://localhost:4317")
                .with_timeout(Duration::from_secs(3))
                .build()
                .expect("Trace exporter should initialize."),
        )
        .build()
}