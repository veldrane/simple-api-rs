use crate::prelude::*;

#[derive(Clone)]
pub struct Metrics {
    pub post_articles_count: Counter,
    pub get_articles_count: Counter,
}

impl Metrics {
    pub fn new(registry: &Registry) -> Self {
        let post_articles_count = Counter::with_opts(Opts::new(
            "post_articles_requests",
            "Total number of handled POST articles requests",
        )).unwrap();

        let get_articles_count = Counter::with_opts(Opts::new(
            "get_articles_requests",
            "Total number of handled GET articles requests",
        )).unwrap();

        registry.register(Box::new(post_articles_count.clone())).unwrap();
        registry.register(Box::new(get_articles_count.clone())).unwrap();


        Self { post_articles_count,
               get_articles_count }
    }

    pub fn build() -> Metrics {
        let registry = Registry::new();
        Metrics::new(&registry)
    }
}