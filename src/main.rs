use std::sync::Arc;

use poem::{ Server, listener::TcpListener};
use simple_api_rs::{ config::Config, app, articles::ArticleList, articles::ArticleStore, exporter::Metrics};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {

    let Config { port, addr, log, .. } = Config::default();

    //log.info(format!("Starting server on port: {}...", port)).await;

    let store = ArticleStore::new(&ArticleList::new());
    let app = app::builder(store.clone(),Arc::new(log));

    Server::new(TcpListener::bind(format!("{}:{}", addr, port)))
        .run(app)
        .await
}