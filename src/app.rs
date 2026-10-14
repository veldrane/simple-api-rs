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
    let tracer = otel::init();

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
            // only this route needs auth, so the middleware is attached
            // to the single endpoint rather than the whole app
            path: "/api/v1/article",
            handler: post(post_article).with(Auth::new()).boxed(),
        },
    ];

    // Order matters here: `.with()` wraps around whatever came before it,
    // so whichever middleware is applied LAST runs FIRST per request.
    // OpenTelemetryTracing must be outermost — it's what creates the root
    // span and makes it the ambient OTel context for everything inside it.
    // fault_inject (and any handler-level tracing::trace_span!) only nests
    // correctly under ONE shared trace if it runs *inside* that context.
    // Swapping this order silently splits every request into two
    // unrelated traces instead of one (verified in tests/otel_trace_shape.rs).
    let api =  routes
    .into_iter()
    .fold(Route::new(), |app, def| app.at(def.path, def.handler))
    .with_result(fault_inject)
    .with( OpenTelemetryTracing::new(tracer.clone()));


    let route=  Route::new()
        .nest("/", api)
        .nest("/metrics", exporter)
        .data(state);

    log.info("Application initialized".into()).await;
    log.info(format!("Starting server on {}:{}", config.addr, config.port)).await;

    route
}


async fn init_fault_inject(config: &Config, log: Arc<Logger>) -> Result<FaultInject, FaultInjectError> {

    FaultInject::try_from(&config.fault_inject)
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
            .with_delay(Duration::from_millis(value.min_delay), Duration::from_millis(value.max_delay))
            .with_timeout(std::time::Duration::from_secs(value.timeout.unwrap_or(2)))
            .with_status(StatusCode::from_u16(value.status_on_error).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        )
    }
}


pub trait EndpointExtOpt: EndpointExt + Sized {
    fn with_opt<T>(self, mw: Option<T>) -> EitherEndpoint<Self, T::Output>
    where
        T: Middleware<Self::Endpoint>,
    {
        match mw {
            Some(mw) => EitherEndpoint::B(self.with(mw)),
            None => EitherEndpoint::A(self),
        }
    }

    fn with_result<T, E>(self, mw: std::result::Result<T, E>) -> EitherEndpoint<Self, T::Output>
    where
        T: Middleware<Self::Endpoint>,
    {
        match mw {
            Ok(mw) => EitherEndpoint::B(self.with(mw)),
            Err(_) => EitherEndpoint::A(self),
        }
    }

}

// blanket impl pro všechny endpointy
impl<T> EndpointExtOpt for T where T: EndpointExt + Sized {}

