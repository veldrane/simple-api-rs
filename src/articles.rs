use poem::{ handler, web::{ Data, Json, Path}, Body, IntoResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{app::AppState, auth::Token, exporter::Metrics};


#[derive(Clone)]
pub struct ArticleStore(Arc<RwLock<ArticleList>>);


impl ArticleStore {

    pub fn new(article_list: &ArticleList) -> Self {
        ArticleStore(Arc::new(RwLock::new(article_list.clone())))
    }

    pub async fn with_read<T> (&self, f: impl FnOnce(&ArticleList) -> T) -> T {
        let guard = self.0.read().await;
        f(& guard)
    }

    pub async fn with_write<T> (&self, f: impl FnOnce(&mut ArticleList) -> T) -> T {
        let mut guard = self.0.write().await;
        f(&mut guard)
    }
}
pub enum GeneralResponse<T> {
    Ok(Json<T>),
    Created,
    NotFound,
    Busy,
    BadRequest,
    Forbidden,
    InternalError,
}

impl<T: Serialize + Send> IntoResponse for GeneralResponse<T>
{
    fn into_response(self) -> poem::Response {
        match self {
            GeneralResponse::Ok(json) => json.into_response(),
            GeneralResponse::Created => poem::Response::builder()
                .status(poem::http::StatusCode::CREATED)
                .body(serde_json::json!({"message": "Article created"}).to_string()),
            GeneralResponse::NotFound => poem::Response::builder()
                .status(poem::http::StatusCode::NOT_FOUND)
                .body(serde_json::json!({"error": "Article not found"}).to_string()),
            GeneralResponse::Busy => poem::Response::builder()
                .status(poem::http::StatusCode::SERVICE_UNAVAILABLE)
                .body(serde_json::json!({"error": "Service is busy"}).to_string()),
            GeneralResponse::BadRequest => poem::Response::builder()
                .status(poem::http::StatusCode::BAD_REQUEST)
                .body(serde_json::json!({"error": "Bad request"}).to_string()),
            GeneralResponse::Forbidden => poem::Response::builder()
                .status(poem::http::StatusCode::FORBIDDEN)
                .body(serde_json::json!({"error": "Access denied"}).to_string()),
            GeneralResponse::InternalError => poem::Response::builder()
                .status(poem::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(serde_json::json!({"error": "Internal server error"}).to_string()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Article {
    #[serde(skip_deserializing)]
    id: u32,
    pub title: String,
    pub author: String,
    pub description: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArticleList {
    pub articles: Vec<Article>,
}

impl ArticleList {
    pub fn new() -> Self {
        let articles = Vec::new();
        ArticleList { articles }
    }

    pub fn add(&mut self, mut article: Article, metrics: &Metrics) -> u32 {
        let id = (self.articles.len() + 1) as u32;
        article.id = id;

        self.articles.push(article);
        metrics.posted_articles.inc();
        id
    }

    pub fn get(&self, id: u32, metrics: &Metrics) -> Option<&Article> {
        let article = self.articles.iter().find(|&article| article.id == id);
        metrics.get_articles.inc();
        article
    }

    pub fn default_example() -> Self {
        let mut articles = Vec::new();        
        let article = Article {
                    id: 1,
                    title: String::from("First Article"),
                    author: String::from("Author One"),
                    description: String::from("This is the first article."),
                    content: String::from("Content of the first article.")
                };
        
        articles.push(article);
        ArticleList { articles: articles }
    }

}

#[handler]
pub async fn get_articles(state: Data<&AppState>) -> GeneralResponse<ArticleList> {

    let AppState { store, log, metrics } = *state;


    log.info(format!("Fetching all articles....")).await;

    let articles = store.with_read(|l| l.clone()).await;
    metrics.get_articles.inc();
    GeneralResponse::Ok(Json(articles))
}

#[handler]
pub async fn get_article_by_id(state: Data<&AppState>, Path(id): Path<u32>) -> GeneralResponse<Article> {

    let AppState { store, log, metrics } = *state;

    log.info(format!("Fetching article by id: {}...", id)).await;

    let article = store.with_read(|l| {
            match l.get(id, metrics) {
                Some(article) => Some(article.clone()),
                None => None,
            }
        });

    match article.await {
        Some(article) => GeneralResponse::Ok(Json(article)),
        None => GeneralResponse::NotFound,
    }


}

#[handler]
pub async fn post_article(state: Data<&AppState>, body: Body, _token: Token) -> GeneralResponse<Article> {

    //if token.0.is_empty() {
    //    return GeneralResponse::Forbidden;
    //}

    //match token.validate_token().await {
    //    Ok(_) => (),
    //    Err(_) => return GeneralResponse::Forbidden,
    //}

    //println!("Received token: {}", token.0);

    let AppState { store, log, metrics } = *state;

    log.info(format!("Creating new article...")).await;
    let data = match body.into_bytes().await {
        Ok(data) => data,
        Err(_) => return GeneralResponse::BadRequest,
    };

    let article = match serde_json::from_slice::<Article>(&data) {
        Ok(article) => article,
        Err(_) => return GeneralResponse::BadRequest,
    };


    store.with_write(|l| l.add(article, metrics)).await;
    GeneralResponse::Created
}

