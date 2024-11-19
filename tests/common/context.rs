use http::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use tonic::Request;

use commerce_v2::common::get_env_var_str;

pub struct TestContext {
    auth_url: String,
    client_id: String,
    client_secret: String,
    header_value: String,
    user_id: String,
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    access_token: String,
}

impl TestContext {
    pub fn from_env() -> Self {
        Self {
            auth_url: get_env_var_str("TEST_AUTH_URL"),
            client_id: get_env_var_str("TEST_CLIENT_ID"),
            client_secret: get_env_var_str("TEST_CLIENT_SECRET"),
            user_id: get_env_var_str("TEST_USER_ID"),
            header_value: String::default(),
        }
    }

    pub fn user_id(&self) -> String {
        self.user_id.clone()
    }

    pub async fn auth_req<T>(&mut self, mut request: T) -> Request<T> {
        if self.header_value.is_empty() {
            self.header_value = self.get_access_token().await;
        }
        let mut request = Request::new(request);
        request.metadata_mut().insert(
            AUTHORIZATION.as_str(),
            format!("Bearer {}", self.header_value.clone())
                .parse()
                .unwrap(),
        );
        request
    }

    async fn get_access_token(&self) -> String {
        let client = reqwest::Client::new();
        let form = [
            ("grant_type", "client_credentials"),
            ("scope", "openid profile"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let res: AuthResponse = client
            .post(self.auth_url.clone())
            .header(CONTENT_TYPE, "application/x-www-form-urlencode")
            .form(&form)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        res.access_token
    }
}
