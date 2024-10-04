pub mod auth;
mod cors;
pub mod query;
mod trace;

pub use cors::init_cors_layer;
pub use trace::init_trace_layer;

pub fn get_env_var(var: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| {
        panic!("ERROR: Missing environment variable '{var}'")
    })
}
