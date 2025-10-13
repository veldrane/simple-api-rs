use simple_api_rs::{config::{ConfigLoader, FileConfigLoader}, prelude::*};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {

    let config = build_config();

    println!("Config dump: {:?}", config);
    let app: AddDataEndpoint<Route, AppState> = app::builder(&config).await;

    Server::new(TcpListener::bind(format!("{}:{}", config.addr, config.port)))
        .run(app)
        .await

}

pub fn build_config() -> Config {

    let args = Args::new();
    let config = match args.config {
        Some(c) => {
            let loader = FileConfigLoader { path:  c.to_string()};
            loader.load()
        },
        None => Config::default(),
    };
    config
}