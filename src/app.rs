use poem::{
    get,
    Route,
    endpoint::{BoxEndpoint, EndpointExt},
};

use crate::hello::{hello, hello2};

type DynHandler = BoxEndpoint<'static, poem::Response>;

// 3) Struktura popisující jednu routu
struct RouteDef {
    path: &'static str,
    handler: DynHandler,
}

pub fn build_app() -> Route {
    let routes: Vec<RouteDef> = vec![
        RouteDef {
            path: "/api/v1/hello/:name",
            handler: get(hello).boxed(),         // <– EndpointExt::boxed()
        },
        RouteDef {
            path: "/api/v1/hello2/:name",
            handler: get(hello2).boxed(),         // <– EndpointExt::boxed()
        },
        // můž
        // můžete přidat další RouteDef sem…
    ];
    // skládáme Route::new() .at(path, handler) pro každou definici
    routes
        .into_iter()
        .fold(Route::new(), |app, def| app.at(def.path, def.handler))
}