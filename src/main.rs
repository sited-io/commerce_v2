use std::sync::Arc;
use tonic::transport::Server;

use commerce_v2::common::auth::Auth;
use commerce_v2::common::{
    get_env_var_int, get_env_var_str, init_cors_layer, init_trace_layer,
};
use commerce_v2::prisma::new_client_with_url;
use commerce_v2::{
    CommerceRepository, CommerceService, FileService, StripeService, Subscriber,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize logging
    tracing_subscriber::fmt::init();

    // get environment variables
    let host = get_env_var_str("HOST");
    let database_url = get_env_var_str("DATABASE_URL");
    let jwks_host = get_env_var_str("JWKS_HOST");
    let jwks_url = get_env_var_str("JWKS_URL");
    let nats_user = get_env_var_str("NATS_USER");
    let nats_password = get_env_var_str("NATS_PASSWORD");
    let nats_host = get_env_var_str("NATS_HOST");
    let s3_bucket_name = get_env_var_str("S3_BUCKET_NAME");
    let s3_bucket_endpoint = get_env_var_str("S3_BUCKET_ENDPOINT");
    let s3_access_key_id = get_env_var_str("S3_ACCESS_KEY_ID");
    let s3_secret_access_key = get_env_var_str("S3_SECRET_ACCESS_KEY");
    let s3_max_allowed_image_size_bytes =
        get_env_var_int("S3_MAX_ALLOWED_IMAGE_SIZE_BYTES");
    let s3_base_url = get_env_var_str("S3_BASE_URL");
    let stripe_secret_key = get_env_var_str("STRIPE_SECRET_KEY");
    let default_user_quota_max_allowed_size_bytes =
        get_env_var_int("DEFAULT_USER_QUOTA_MAX_ALLOWED_SIZE_BYTES");
    let default_platform_fee_percent =
        get_env_var_int("DEFAULT_PLATFORM_FEE_PERCENT");
    let default_minimum_platform_fee_cent =
        get_env_var_int("DEFAULT_MINIMUM_PLATFORM_FEE_CENT");

    // initialize JWKS verifier
    let auth = Auth::new(&jwks_host, &jwks_url);

    // initialize repository
    let db_client = Arc::new(new_client_with_url(&database_url).await?);
    let repository = CommerceRepository::new(db_client);

    // initialize file service
    let file_service = FileService::init(
        s3_bucket_name,
        s3_bucket_endpoint,
        s3_access_key_id,
        s3_secret_access_key,
        s3_max_allowed_image_size_bytes,
    )
    .await;

    // initialize Stripe service
    let stripe_service = StripeService::init(stripe_secret_key);

    // initialize NATS client
    let nats_client = async_nats::ConnectOptions::new()
        .user_and_password(nats_user, nats_password)
        .connect(nats_host)
        .await?;

    // initialize website subscriber
    let subscriber = Subscriber::new(nats_client, repository.clone());

    // initialize gRPC service
    let service = CommerceService::init(
        auth,
        repository,
        file_service,
        stripe_service,
        s3_base_url,
        default_user_quota_max_allowed_size_bytes,
        default_platform_fee_percent as u32,
        default_minimum_platform_fee_cent as u32,
    );

    // initialize layers
    let trace_layer = init_trace_layer();
    let cors_layer = init_cors_layer();

    let (subscriber_task, server_task) = tokio::join!(
        tokio::spawn(async move {
            tracing::log::info!("NATS subscriber listening");
            subscriber.subscribe().await
        }),
        tokio::spawn(async move {
            tracing::log::info!("gRPC+web server listening on {}", host);
            Server::builder()
                .accept_http1(true)
                .layer(trace_layer)
                .layer(cors_layer)
                .add_service(tonic_web::enable(service))
                .serve(host.parse().unwrap())
                .await
        })
    );

    subscriber_task?;
    server_task??;

    Ok(())
}
