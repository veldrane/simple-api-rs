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
            min_delay: Duration::from_micros(0),
            max_delay: Duration::from_micros(0),
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
        .with_delay(std::time::Duration::from_micros(50), std::time::Duration::from_micros(100))
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
        
        //let cx = tracing::Span::current().context();
        //let span= req.extensions().get::<tracing::Span>().unwrap().clone();
        //let instrumented = req.data::<SdkTracer>().unwrap();
        //let span = instrumented.in_current_span().clone();
        //let _e = span.enter();

        let span = req.data::<SdkTracer>().unwrap().span_builder("app.call")
            .with_attributes(vec![
                KeyValue::new("part_of", "fault_inject"),
            ]);

        let _ot_span = span.start(req.data::<SdkTracer>().unwrap());

        
        
        let span = tracing::trace_span!("fault_inject.call");
        let _e = span.enter();

        let mut rng = rand::rngs::OsRng;
        let data = req.data::<AppState>().unwrap();
        let log = &data.log.clone();



        log.info("Hello from FaultInject middleware".into()).await;
        // 1) Umělá latence
        let extra: Duration = if self.cfg.max_delay > self.cfg.min_delay {
            let spread = self.cfg.max_delay - self.cfg.min_delay;
            self.cfg.min_delay + Duration::from_micros(
                rng.gen_range(0..=spread.as_micros() as u64)
            )
        } else {
            Duration::from_micros(0)
        };
        
        //let span = tracing::trace_span!(
        //    "fault.delay",
        //    fault_injected = true,
        //    reason = "delay",
        //    delay_ms = extra.as_micros() as u64
        //);

        //let _eds = delay_span.enter();


        async {  
            log.info(format!("Injecting delay of {:?}us", extra.as_millis())).await;
            sleep(extra).await;
        }
        .await;

        tracing::trace!(delay_finish = true);
        drop(_e);

        //log.info(format!("Injecting delay of {:?}ms", extra.as_millis())).await;
        //sleep(extra).await;

        //drop(_eds);
        // 2) Fuzzy chyba
        //if rng.gen_range(0.0f32..1.0f32) < self.cfg.error_rate {
        //    return Ok(GeneralResponse::<()>::InternalError.into_response());
        //}


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
