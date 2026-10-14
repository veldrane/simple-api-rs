use crate::prelude::*;
use oidc_jwt_validator::{ValidationSettings, Validator, cache::Strategy};

const OIDC_URL: &str = "https://idp.class.syscallx86.com:8443/realms/class.syscallx86.com";

/// Middleware that requires a valid `Authorization: Bearer <token>` header,
/// checked against `OIDC_URL`, before the wrapped endpoint runs. Attach it
/// to individual routes with `.with(Auth::new())` (see app.rs) rather than
/// to the whole app, so only the endpoints that need it pay for it.
#[derive(Clone)]
pub struct Auth {
    oidc_url: &'static str,
}

impl Auth {
    pub fn new() -> Self {
        Self { oidc_url: OIDC_URL }
    }
}

impl<E: Endpoint> Middleware<E> for Auth {
    type Output = AuthEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        AuthEndpoint { inner: ep, cfg: self.clone() }
    }
}

pub struct AuthEndpoint<E> {
    inner: E,
    cfg: Auth,
}

impl<E: Endpoint> Endpoint for AuthEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let token = extract_bearer_token(&req)
            .ok_or_else(|| Error::from_string("missing token", StatusCode::FORBIDDEN))?;

        validate_token(self.cfg.oidc_url, &token)
            .await
            .map_err(|_| Error::from_string("invalid token", StatusCode::FORBIDDEN))?;

        self.inner.call(req).await.map(IntoResponse::into_response)
    }
}

fn extract_bearer_token(req: &Request) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(str::to_string)
}

async fn validate_token(oidc_url: &str, token: &str) -> std::result::Result<(), ()> {
    let http_client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(2))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|_| ())?;

    let settings = ValidationSettings::new();

    let validator = Validator::new(oidc_url, http_client, Strategy::Automatic, settings)
        .await
        .map_err(|_| ())?;

    match validator.validate::<()>(token).await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Token validation failed: {}", e);
            Err(())
        }
    }
}
