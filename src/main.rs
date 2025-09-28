use simple_api_rs::prelude::*;



#[tokio::main]
async fn main() -> Result<(), std::io::Error> {

    let config = Config::default();
    let app = app::builder(&config).await;

    Server::new(TcpListener::bind(format!("{}:{}", config.addr, config.port)))
        .run(app)
        .await
}