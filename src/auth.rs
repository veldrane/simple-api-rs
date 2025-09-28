#![allow(unused_imports, unused_variables, unused_mut)]

use oidc_jwt_validator::{cache::Strategy, ValidationSettings, Validator};
use crate::prelude::*;

const OIDC_URL: &str = "https://idp.class.syscallx86.com:8443/realms/class.syscallx86.com";

#[derive(Deserialize, Serialize, Clone)]
pub struct Token( pub String);

impl<'a> FromRequest<'a> for Token {
    async fn from_request(req: &'a Request, _: &mut RequestBody) -> Result<Self> {
        let _token = req
                            .headers()
                            .get("Authorization")
                            .and_then(|value| value.to_str().ok())
                            .and_then(|s| {
                                if s.starts_with("Bearer ") {
                                    s.split_whitespace().nth(1)
                                } else {
                                    None
                                }
                            });
                            //.ok_or_else(|| Error::from_string("missing token", StatusCode::FORBIDDEN))?;
        //Ok(Token(token.to_string()))

        Ok(Token("".to_string()))
    }
}


impl Token {
    pub async fn validate_token(&self) -> Result<(), Error> {

        let http_client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(2))
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

        let mut settings = ValidationSettings::new();

        //settings.set_issuer(&["https://idp.class.syscallx86.com:8443/realms/class.syscallx86.com"]);
        //settings.set_audience(&["account"]);



        let validator = Validator::new(OIDC_URL, http_client, Strategy::Automatic, settings)
            .await
            .unwrap();

        match  validator.validate::<()>(self.0.as_str()).await {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Token validation failed: {} {}", e, self.0);
                Err(Error::from_string("invalid token", StatusCode::FORBIDDEN))
            }
        }
    }
}