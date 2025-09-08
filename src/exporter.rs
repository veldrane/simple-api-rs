use prometheus::{Opts, Registry, Counter};

#[derive(Clone)]
pub struct Metrics {
    pub posted_articles: Counter,
    pub get_articles: Counter,
}

impl Metrics {
    pub fn new(registry: &Registry) -> Self {
        let posted_articles = Counter::with_opts(Opts::new(
            "posted_articles_requests",
            "Total number of handled POST articles requests",
        )).unwrap();

        let get_articles = Counter::with_opts(Opts::new(
            "get_articles_requests",
            "Total number of handled GET articles requests",
        )).unwrap();

        registry.register(Box::new(posted_articles.clone())).unwrap();
        registry.register(Box::new(get_articles.clone())).unwrap();

        
        Self { posted_articles,
               get_articles }
    }
}