use poem::{ endpoint::{BoxEndpoint, EndpointExt, PrometheusExporter}, get, post, middleware::AddDataEndpoint, Route };
use std::sync::Arc;

use crate::{articles::{ get_article_by_id, post_article }, status::up};
use crate::articles::{ get_articles, ArticleStore };
use crate::logging::Logger;
use crate::exporter::Metrics;

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

pub fn builder(store: ArticleStore, log: Arc<Logger>) -> AddDataEndpoint<Route, AppState> {

    let registry = prometheus::Registry::new();

    let metrics = Metrics::new(&registry);

    let state = AppState { store, log, metrics };

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
    // skládáme Route::new() .at(path, handler) pro každou definici
    let route = routes
        .into_iter()
        .fold(Route::new(), |app, def| app.at(def.path, def.handler))
        .nest("/metrics", PrometheusExporter::new(registry.clone()))
        .data(state)
; // přidáme sdílená data

    route
}