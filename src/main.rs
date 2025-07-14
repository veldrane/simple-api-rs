use poem::{IntoResponse, Route, Server, get, handler, listener::TcpListener, web::Path};
use tokio;

use simple_api_rs::config::Config;


#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {



    let config = Config::default();

    // let app = Route::new().at("/api/v1/hello/:name", get(hello));

    let app = simple_api_rs::app::build_app();
    Server::new(TcpListener::bind(format!("{}:{}", config.addr, config.port)))
        .run(app)
        .await
}