use std::collections::HashMap;
use std::time::Duration;

use http::header::{AUTHORIZATION, HOST};
use http::{HeaderMap, HeaderValue};
use jwtk::jwk::RemoteJwksVerifier;
use serde::Deserialize;
use tonic::metadata::MetadataMap;
use tonic::{Request, Status};

#[allow(unused)]
#[derive(Debug, Clone, Deserialize)]
struct ExtraClaims {
    #[serde(rename = "urn:zitadel:iam:user:metadata")]
    metadata: HashMap<String, String>,
}

pub struct Auth {
    verifier: RemoteJwksVerifier,
}

impl Auth {
    pub fn new(jwks_host: &str, jwks_url: &String) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_str(jwks_host).unwrap());
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            verifier: RemoteJwksVerifier::new(
                jwks_url.to_owned(),
                Some(client),
                Duration::from_secs(120),
            ),
        }
    }

    pub async fn get_user_id<T>(
        &self,
        request: &Request<T>,
    ) -> Result<String, Status> {
        let token = self.get_token(request.metadata())?;

        self.verifier
            .verify::<()>(&token)
            .await
            .map_err(|err| Status::unauthenticated(err.to_string()))?
            .claims()
            .sub
            .clone()
            .ok_or_else(|| {
                tracing::error!("claim 'sub' missing in token");
                Status::unauthenticated("")
            })
    }

    fn get_token(&self, metadata: &MetadataMap) -> Result<String, Status> {
        metadata
            .get(AUTHORIZATION.as_str())
            .and_then(|v| v.to_str().ok())
            .and_then(|header_value| header_value.split_once(' '))
            .map(|(_, token)| token.to_string())
            .ok_or_else(|| {
                tracing::error!(
                    "{} header missing or malformed",
                    AUTHORIZATION,
                );
                Status::unauthenticated("")
            })
    }
}
