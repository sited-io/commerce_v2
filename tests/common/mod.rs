mod cleanup;
#[allow(unused)]
mod context;
#[allow(unused)]
pub mod fixtures;
#[allow(unused)]
pub mod offer;

#[allow(unused)]
pub use cleanup::{cleanup_offers, cleanup_open_multipart_uploads};
pub use context::TestContext;

use rand::{distributions::Alphanumeric, Rng};
use service_apis::sited_io::commerce::v2::commerce_service_client::CommerceServiceClient;
use tonic::transport::Channel;

use commerce_v2::common::get_env_var_str;

pub fn setup_test_context() -> TestContext {
    tracing_subscriber::fmt::init();

    TestContext::from_env()
}

pub async fn setup_commerce_client() -> CommerceServiceClient<Channel> {
    let commerce_url = get_env_var_str("TEST_COMMERCE_URL");
    tracing::info!("Runnning integration test against: {}", commerce_url);

    CommerceServiceClient::connect(commerce_url).await.unwrap()
}

pub async fn setup_nats_client() -> async_nats::Client {
    async_nats::ConnectOptions::new()
        .user_and_password(
            get_env_var_str("NATS_USER"),
            get_env_var_str("NATS_PASSWORD"),
        )
        .connect(get_env_var_str("NATS_HOST"))
        .await
        .unwrap()
}

pub async fn setup() -> (TestContext, CommerceServiceClient<Channel>) {
    (setup_test_context(), setup_commerce_client().await)
}

pub fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
