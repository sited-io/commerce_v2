#[allow(unused)]
mod cleanup;
#[allow(unused)]
mod context;
#[allow(unused)]
pub mod fixtures;
#[allow(unused)]
pub mod offer;

#[allow(unused)]
mod setup;

#[allow(unused)]
pub use cleanup::{cleanup_offers, cleanup_open_multipart_uploads};
pub use context::TestContext;

pub use setup::*;

use rand::{distributions::Alphanumeric, Rng};

pub fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
