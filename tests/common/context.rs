use http::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use tonic::Request;

use commerce_v2::common::get_env_var_str;

pub struct TestContext {
    auth_url: String,
    owner_client_id: String,
    owner_client_secret: String,
    owner_user_id: String,
    owner_header_value: String,
    customer_client_id: String,
    customer_client_secret: String,
    customer_user_id: String,
    customer_header_value: String,
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    access_token: String,
}

impl TestContext {
    pub fn from_env() -> Self {
        Self {
            auth_url: get_env_var_str("TEST_AUTH_URL"),
            owner_client_id: get_env_var_str(
                "TEST_INTEGRATION_TEST_OWNER_CLIENT_ID",
            ),
            owner_client_secret: get_env_var_str(
                "TEST_INTEGRATION_TEST_OWNER_SECRET",
            ),
            owner_user_id: get_env_var_str(
                "TEST_INTEGRATION_TEST_OWNER_USER_ID",
            ),
            owner_header_value: String::default(),
            customer_client_id: get_env_var_str(
                "TEST_INTEGRATION_TEST_CUSTOMER_CLIENT_ID",
            ),
            customer_client_secret: get_env_var_str(
                "TEST_INTEGRATION_TEST_CUSTOMER_SECRET",
            ),
            customer_user_id: get_env_var_str(
                "TEST_INTEGRATION_TEST_CUSTOMER_USER_ID",
            ),
            customer_header_value: String::default(),
        }
    }

    pub fn owner_user_id(&self) -> String {
        self.owner_user_id.clone()
    }

    pub fn customer_user_id(&self) -> String {
        self.customer_user_id.clone()
    }

    pub async fn owner_auth_req<T>(&mut self, mut request: T) -> Request<T> {
        if self.owner_header_value.is_empty() {
            self.owner_header_value = self
                .get_access_token(
                    &self.owner_client_id,
                    &self.owner_client_secret,
                )
                .await;
        }
        let mut request = Request::new(request);
        request.metadata_mut().insert(
            AUTHORIZATION.as_str(),
            format!("Bearer {}", self.owner_header_value.clone())
                .parse()
                .unwrap(),
        );
        request
    }

    pub async fn customer_auth_req<T>(&mut self, mut request: T) -> Request<T> {
        if self.customer_header_value.is_empty() {
            self.customer_header_value = self
                .get_access_token(
                    &self.customer_client_id,
                    &self.customer_client_secret,
                )
                .await;
        }
        let mut request = Request::new(request);
        request.metadata_mut().insert(
            AUTHORIZATION.as_str(),
            format!("Bearer {}", &self.customer_header_value)
                .parse()
                .unwrap(),
        );
        request
    }

    async fn get_access_token(
        &self,
        client_id: &String,
        client_secret: &String,
    ) -> String {
        let client = reqwest::Client::new();
        let form = [
            ("grant_type", "client_credentials"),
            ("scope", "openid profile"),
            ("client_id", client_id),
            ("client_secret", client_secret),
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
