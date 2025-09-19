use poem::{http::StatusCode, Endpoint, IntoResponse, Middleware, Request, Response, Result};
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;
use crate::{app::AppState, articles::GeneralResponse};
use tracing::{info, info_span};

#[derive(Clone)]
pub struct FaultInject {
    pub error_rate: f32,                // 0.0..=1.0
    pub min_delay: Duration,            // minimální přidané zpoždění
    pub max_delay: Duration,            // maximální přidané zpoždění
    pub timeout: Option<Duration>,      // per-request timeout
    pub status_on_error: StatusCode,    // status pro "fuzzy" chybu
}

impl FaultInject {
    pub fn new() -> Self {
        Self {
            error_rate: 0.0,
            min_delay: Duration::from_millis(0),
            max_delay: Duration::from_millis(0),
            timeout: None,
            status_on_error: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn with_error_rate(mut self, p: f32) -> Self { self.error_rate = p; self }
    pub fn with_delay(mut self, min: Duration, max: Duration) -> Self { self.min_delay = min; self.max_delay = max; self }
    pub fn with_timeout(mut self, dur: Duration) -> Self { self.timeout = Some(dur); self }
    pub fn with_status(mut self, status: StatusCode) -> Self { self.status_on_error = status; self }
}

impl<E: Endpoint> Middleware<E> for FaultInject {
    type Output = FaultInjectEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        FaultInjectEndpoint { inner: ep, cfg: self.clone() }
    }
}

#[derive(Clone)]
pub struct FaultInjectEndpoint<E> {
    inner: E,
    cfg: FaultInject,
}


impl<E: Endpoint> Endpoint for FaultInjectEndpoint<E> {

    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        
        let mut rng = rand::rngs::OsRng;
        let data = req.data::<AppState>().unwrap();
        let log = &data.log.clone();



        log.info("Hello from FaultInject middleware".into()).await;
        // 1) Umělá latence
        let extra: Duration = if self.cfg.max_delay > self.cfg.min_delay {
            let spread = self.cfg.max_delay - self.cfg.min_delay;
            self.cfg.min_delay + Duration::from_millis(
                rng.gen_range(0..=spread.as_millis() as u64)
            )
        } else {
            Duration::from_millis(0)
        };
        
        let delay_span = info_span!(
            "fault.delay",
            fault_injected = true,
            reason = "delay",
            delay_ms = extra.as_millis() as u64
        );

        let _eds = delay_span.enter();
        log.info(format!("Injecting delay of {:?}ms", extra.as_millis())).await;
        sleep(extra).await;


        // 2) Fuzzy chyba
        //if rng.gen_range(0.0f32..1.0f32) < self.cfg.error_rate {
        //    return Ok(GeneralResponse::<()>::InternalError.into_response());
        //}

        let process_span = info_span!(
            "endpoint.process",
            fault_injected = true,
            reason = "process",
            endpoint = req.uri().path() as &str
        );

        let _eps = process_span.enter();

        let res = self.inner.call(req).await;

        match res {
            Ok(resp) => {
                let rsp = resp.into_response();
                log.info("Response from endpoints has been ok".into()).await;
                Ok(rsp)
            },
            Err(e) => {
                log.error(format!("FaultInject middleware: {:?}", e)).await;
                Err(e)
            },
        }
    }
}
