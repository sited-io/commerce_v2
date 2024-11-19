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

pub async fn setup() -> (TestContext, CommerceServiceClient<Channel>) {
    tracing_subscriber::fmt::init();

    (
        TestContext::from_env(),
        CommerceServiceClient::connect(get_env_var_str("TEST_COMMERCE_URL"))
            .await
            .unwrap(),
    )
}

pub fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
