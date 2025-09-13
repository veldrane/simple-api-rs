use poem::{ endpoint::{BoxEndpoint, EndpointExt, PrometheusExporter}, get, post, middleware::AddDataEndpoint, Route };
use std::sync::Arc;

use crate::{articles::{ get_article_by_id, post_article }, config::Config, fault_inject, status::up};
use crate::articles::{ get_articles, ArticleStore, ArticleList };
use crate::logging::Logger;
use crate::exporter::Metrics;
use crate::fault_inject::FaultInject;


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
    .with(fault_inject);

    let route=  Route::new()
        .nest("/", api_only)
        .nest("/metrics", PrometheusExporter::new(registry.clone()))
        .data(state);

    log.info("Application initialized".into()).await;
    log.info(format!("Starting server on {}:{}", config.addr, config.port)).await;


    route
}