use tokio::time::sleep;
use crate::prelude::*;


#[derive(Clone)]
pub struct FaultInject {
    pub error_rate: f32,                // 0.0..=1.0
    pub min_delay: Duration,            // minimální přidané zpoždění
    pub max_delay: Duration,            // maximální přidané zpoždění
    pub timeout: Option<Duration>,      // per-request timeout
    pub status_on_error: StatusCode,    // status pro "fuzzy" chybu
}

pub enum FaultInjectError {
    InvalidErrorRate,
    InvalidDelay,
    InvalidTimeout,
    InvalidStatusCode,
}


impl Into<String> for FaultInjectError {
    fn into(self) -> String {
        match self {
            FaultInjectError::InvalidErrorRate => "Invalid error rate".into(),
            FaultInjectError::InvalidDelay => "Invalid delay range".into(),
            FaultInjectError::InvalidTimeout => "Invalid timeout".into(),
            FaultInjectError::InvalidStatusCode => "Invalid status code".into(),
        }
    }
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

impl Default for FaultInject {
    fn default() -> Self {

        FaultInject::new()
        .with_error_rate(0.1)
        .with_delay(std::time::Duration::from_millis(50), std::time::Duration::from_millis(100))
        .with_timeout(std::time::Duration::from_secs(2))
        .with_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR)
    }
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
        // A plain tracing span is enough: poem's `OpenTelemetryTracing`
        // middleware already made its request-span the ambient OTel
        // context, so `tracing-opentelemetry` links this span under it
        // automatically (same trace_id) — no manual OTel span needed here.
        // The whole body is wrapped with `.instrument(span)` rather than
        // `span.enter()` because this function `.await`s (entering a span
        // and holding the guard across an `.await` point corrupts span
        // data once requests interleave on the same worker thread).
        let span = tracing::trace_span!("fault_inject.call");

        async move {
            let mut rng = rand::rngs::OsRng;
            let data = req.data::<AppState>().unwrap();
            let log = &data.log.clone();

            log.info("Hello from FaultInject middleware".into()).await;

            // 1) Umělá latence
            let extra: Duration = if self.cfg.max_delay > self.cfg.min_delay {
                let spread = (self.cfg.max_delay - self.cfg.min_delay).mul_f32(1.0 + self.cfg.error_rate * 2.0);

                self.cfg.min_delay + Duration::from_millis(
                    rng.gen_range(0..=spread.as_millis() as u64)
                )
            } else {
                Duration::from_millis(0)
            };

            if extra > self.cfg.max_delay {
                log.info(format!("Injecting max delay of {:?}ms", self.cfg.max_delay.as_millis())).await;
                sleep(self.cfg.max_delay).await;
                return Err(Error::from_string("Request reach maximum timeout\n", self.cfg.status_on_error));
            }

            log.info(format!("Injecting delay of {:?}ms", extra.as_millis())).await;
            sleep(extra).await;

            tracing::trace!(delay_finish = true);

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
        .instrument(span)
        .await
    }
}
