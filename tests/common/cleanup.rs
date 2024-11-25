use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use commerce_v2::common::get_env_var_str;
use commerce_v2::prisma::{new_client_with_url, order};
use service_apis::sited_io::commerce::v2::{
    RemoveFileFromOfferRequest, RemoveImageFromOfferRequest,
};
use tonic::transport::Channel;

use service_apis::sited_io::commerce::v2::{
    commerce_service_client::CommerceServiceClient, DeleteOfferRequest,
    ListOffersRequest,
};

use super::context::TestContext;

pub async fn cleanup_offers(
    ctx: &mut TestContext,
    commerce_client: &mut CommerceServiceClient<Channel>,
) {
    let db = new_client_with_url(&get_env_var_str("DATABASE_URL"))
        .await
        .unwrap();

    let res = commerce_client
        .list_offers(ListOffersRequest::default())
        .await
        .unwrap()
        .into_inner();

    if res.offers.len() > 0 {
        tracing::warn!("CLEANUP: will delete {} offers", res.offers.len());

        for offer in res.offers {
            db.order()
                .delete_many(vec![order::offer_id::equals(
                    offer.offer_id.clone(),
                )])
                .exec()
                .await
                .unwrap();
            for image in offer.images {
                let req = ctx
                    .owner_auth_req(RemoveImageFromOfferRequest {
                        offer_image_id: image.offer_image_id,
                        offer_id: offer.offer_id.clone(),
                    })
                    .await;
                commerce_client.remove_image_from_offer(req).await.unwrap();
            }
            for file in offer.files {
                let req = ctx
                    .owner_auth_req(RemoveFileFromOfferRequest {
                        offer_file_id: file.offer_file_id.clone(),
                    })
                    .await;
                commerce_client.remove_file_from_offer(req).await.unwrap();
            }
            let req = ctx
                .owner_auth_req(DeleteOfferRequest {
                    offer_id: offer.offer_id,
                })
                .await;
            commerce_client.delete_offer(req).await.unwrap();
        }
    }
}

pub async fn cleanup_open_multipart_uploads() {
    let test_user_id = get_env_var_str("TEST_INTEGRATION_TEST_OWNER_USER_ID");
    let bucket = get_env_var_str("S3_BUCKET_NAME");
    let credentials = Credentials::new(
        get_env_var_str("S3_ACCESS_KEY_ID"),
        get_env_var_str("S3_SECRET_ACCESS_KEY"),
        None,
        None,
        "Static",
    );

    let config = aws_config::defaults(BehaviorVersion::v2024_03_28())
        .credentials_provider(credentials)
        .region(Region::new("auto"))
        .endpoint_url(get_env_var_str("S3_BUCKET_ENDPOINT"))
        .load()
        .await;

    let client = Client::new(&config);

    let res = client
        .list_multipart_uploads()
        .bucket(bucket.clone())
        .send()
        .await
        .unwrap();

    for upload in res.uploads() {
        let user_id = upload.key().unwrap().split('/').next().unwrap();
        if user_id == test_user_id {
            client
                .abort_multipart_upload()
                .bucket(bucket.clone())
                .key(upload.key().unwrap())
                .upload_id(upload.upload_id().unwrap())
                .send()
                .await
                .unwrap();
        }
    }
}
