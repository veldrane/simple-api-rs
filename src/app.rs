use crate::prelude::*;

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
    let fault_inject = init_fault_inject(&config, log.clone()).await;
    let tracer = init_tracer().await;



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
    .with(OpenTelemetryTracing::new(tracer.clone()))
    .with(fault_inject)
    .data(tracer.clone());


    let route=  Route::new()
        .nest("/", api)
        .nest("/metrics", exporter)
        .data(state);

    log.info("Application initialized".into()).await;
    log.info(format!("Starting server on {}:{}", config.addr, config.port)).await;

    route
}


async fn init_tracer() -> SdkTracer {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let provider = SdkTracerProvider::builder()
        .with_resource(Resource::builder().with_service_name("simple-api").build())
        .with_batch_exporter(
            opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_protocol(Protocol::Grpc)
                .with_endpoint("http://localhost:4317")
                .with_timeout(Duration::from_secs(3))
                .build()
                .expect("Trace exporter should initialize."),
        )
        .build();

    let tracer = provider.tracer("simple-api-trace");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());
    let subscriber = tracing_subscriber::Registry::default().with(otel_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Unable to set global subscriber.");

    tracer
}

async fn init_fault_inject(config: &Config, log: Arc<Logger>) -> FaultInject {

    match FaultInject::try_from(&config.fault_inject) {
        Ok(fi) => return fi,
        Err(e) => {
            let m: String = e.into();
            log.error(format!("Failed to initialize fault injection middleware: {}", m)).await;
            return fault_inject::FaultInject::default()
        }
    };
}


impl TryFrom<&FaultInjectConfig> for FaultInject {

    type Error = FaultInjectError;

fn try_from(value: &FaultInjectConfig) -> std::result::Result<Self, FaultInjectError> {
    
    if value.error_rate < 0.0 || value.error_rate > 1.0 {
        return Err(FaultInjectError::InvalidErrorRate);
    }
    if value.min_delay > value.max_delay {
        return Err(FaultInjectError::InvalidDelay);
    }
    if let Some(timeout) = value.timeout {
        if timeout == 0 {
            return Err(FaultInjectError::InvalidTimeout);
        }
    }

    Ok(
        FaultInject::new()
        .with_error_rate(value.error_rate)
        .with_delay(Duration::from_micros(value.min_delay), Duration::from_micros(value.max_delay))
        .with_timeout(std::time::Duration::from_secs(value.timeout.unwrap_or(2)))
        .with_status(StatusCode::from_u16(value.status_on_error).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
    )
}
}
