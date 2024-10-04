use http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use http::{HeaderName, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

pub fn init_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_headers([
            AUTHORIZATION,
            ACCEPT,
            CONTENT_TYPE,
            HeaderName::from_static("grpc-status"),
            HeaderName::from_static("grpc-message"),
            HeaderName::from_static("x-grpc-web"),
            HeaderName::from_static("x-user-agent"),
        ])
        .allow_methods([Method::POST])
        .allow_origin(AllowOrigin::any())
        .allow_private_network(true)
}
