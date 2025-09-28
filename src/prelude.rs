// Re-export commonly used items for easier access
pub use crate::{
    articles::{get_article_by_id, get_articles, post_article, ArticleList, ArticleStore},
    config::{ Config, FaultInjectConfig},
    app,
    {app::AppState, auth::Token},
    exporter::Metrics,
    fault_inject::{self, FaultInject, FaultInjectError},
    logging::Logger,
    status::up,
};

// Poem framework
pub use poem::{
    endpoint::{BoxEndpoint, EndpointExt, PrometheusExporter},
    get, post, middleware::{AddDataEndpoint, OpenTelemetryTracing},
    Response, Route,
    http::StatusCode,
    handler, web::{ Data, Json, Path}, Body, IntoResponse, Request, FromRequest, RequestBody, Result, Error,
    Middleware, Endpoint, Server, 
    listener::TcpListener,
};


// OpenTelemetry
pub use opentelemetry::{global, trace::TracerProvider as _, KeyValue, {trace::{FutureExt, TraceContextExt, Tracer}}};
pub use opentelemetry_otlp::{Protocol, WithExportConfig, };
pub use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{SdkTracerProvider, SdkTracer},
    Resource,
};

// Tracing
pub use tracing_subscriber::layer::SubscriberExt;

//Serde
pub use serde::{Deserialize, Serialize};

//Others
pub use rand::Rng;
pub use std::time::Duration;
pub use std::sync::Arc;

// Prometheus
pub use prometheus::{Opts, Registry, Counter};