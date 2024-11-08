use std::sync::Arc;
use tonic::transport::Server;

use commerce_v2::common::auth::Auth;
use commerce_v2::common::{get_env_var, init_cors_layer, init_trace_layer};
use commerce_v2::prisma::new_client_with_url;
use commerce_v2::{CommerceRepository, CommerceService, Publisher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize logging
    tracing_subscriber::fmt::init();

    // get environment variables
    let host = get_env_var("HOST");
    let database_url = get_env_var("DATABASE_URL");
    let jwks_host = get_env_var("JWKS_HOST");
    let jwks_url = get_env_var("JWKS_URL");
    let nats_user = get_env_var("NATS_USER");
    let nats_password = get_env_var("NATS_PASSWORD");
    let nats_host = get_env_var("NATS_HOST");

    // initialize JWKS verifier
    let auth = Auth::new(&jwks_host, &jwks_url);

    // initialize repository
    let db_client = Arc::new(new_client_with_url(&database_url).await?);
    let repository = CommerceRepository::new(db_client);

    // initialize publisher
    let publisher = Publisher::init(nats_user, nats_password, nats_host).await;

    // initialize gRPC service
    let service = CommerceService::init(auth, repository, publisher);

    // initialize layers
    let trace_layer = init_trace_layer();
    let cors_layer = init_cors_layer();

    tracing::log::info!("gRPC+web server listening on {}", host);

    Server::builder()
        .accept_http1(true)
        .layer(trace_layer)
        .layer(cors_layer)
        .add_service(tonic_web::enable(service))
        .serve(host.parse().unwrap())
        .await?;

    Ok(())
}
