use poem::{ Server, listener::TcpListener};
use std::sync::{Arc, RwLock};
use simple_api_rs::{ config::Config, app, articles::ArticleList, articles::ArticleStore};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {

    let articles = ArticleList::new();

    let store: ArticleStore = Arc::new(RwLock::new(articles));



    let config = Config::default();

    // let app = Route::new().at("/api/v1/hello/:name", get(hello));

    let app = app::builder(store.clone());
    Server::new(TcpListener::bind(format!("{}:{}", config.addr, config.port)))
        .run(app)
        .await
}