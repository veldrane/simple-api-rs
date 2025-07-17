use poem::{ handler, web::{ Data, Json, Path}, Body, IntoResponse};
use serde::{Deserialize, Serialize};
use std::{sync::{Arc, RwLock}};



pub type ArticleStore = Arc<RwLock<ArticleList>>;
pub enum GeneralResponse<T> {
    Ok(Json<T>),
    Created,
    NotFound,
    Busy,
    BadRequest,
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

    pub fn default() -> Self {


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

    pub fn add(&mut self, mut article: Article) -> u32 {

        let id = (self.articles.len() + 1) as u32;
        article.id = id;
        self.articles.push(article);

        id
    }

    pub fn get(&self, id: u32) -> Option<Article> {

        let article = self.articles.iter().find(|&article| article.id == id).cloned();
        article
    }
}

#[handler]
pub async fn get_articles(state: Data<&ArticleStore>) -> GeneralResponse<ArticleList> {

    let articles = match state.read() {
        Ok(guard) => guard,
        Err(_) => return GeneralResponse::Busy
    };
    GeneralResponse::Ok(Json(articles.clone()))
}

#[handler]
pub async fn get_article_by_id(state: Data<&ArticleStore>, Path(id): Path<u32>) -> GeneralResponse<Article> {

    let articles = match state.read() {
        Ok(guard) => guard,
        Err(_) => return GeneralResponse::Busy
    };

    match articles.get(id) {
        Some(article) => GeneralResponse::Ok(Json(article)),
        None => GeneralResponse::NotFound,
    }
}

#[handler]
pub async fn post_article(state: Data<&ArticleStore>, body: Body) -> GeneralResponse<Article> {

    let data = match body.into_bytes().await {
        Ok(data) => data,
        Err(_) => return GeneralResponse::BadRequest,
    };

    let article = match serde_json::from_slice::<Article>(&data) {
        Ok(article) => article,
        Err(_) => return GeneralResponse::BadRequest,
    };

    let mut articles = match state.write() {
        Ok(guard) => guard,
        Err(_) => return GeneralResponse::Busy
    };

    articles.add(article.clone());
    GeneralResponse::Created
}