use rand::RngCore;
use service_apis::sited_io::commerce::v2::{
    CompleteMultipartUploadRequest, InitiateMultipartUploadRequest,
    PutMultipartChunkRequest, RemoveFileFromOfferRequest,
};

mod common;

use common::offer::{create_offer, delete_offer, get_offer};
use common::random_string;

const MOCK_DATA_SIZE: usize = 5252880; // little more than 5MiB

#[tokio::test]
async fn offer_file_multipart_test() {
    let (mut ctx, mut commerce_client) = common::setup().await;

    common::cleanup_open_multipart_uploads().await;
    common::cleanup_offers(&mut ctx, &mut commerce_client).await;

    // Create Offer : ok
    let offer = create_offer(&mut ctx, &mut commerce_client).await;

    // Check Offer : empty files : ok
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.files.is_empty());

    let mut data = vec![0u8; MOCK_DATA_SIZE];
    rand::thread_rng().fill_bytes(&mut data);

    // Initiate Multipart Upload : ok
    let req = ctx
        .owner_auth_req(InitiateMultipartUploadRequest {
            file_name: random_string(8),
            offer_id: offer.offer_id.clone(),
            total_size_bytes: MOCK_DATA_SIZE as u64,
            content_type: Some("image/jpeg".to_string()),
            ordering: Some(1),
        })
        .await;
    let res = commerce_client
        .initiate_multipart_upload(req)
        .await
        .unwrap()
        .into_inner();
    let offer_file_id = res.offer_file_id;
    let upload_id = res.upload_id;

    // Put Multipart Chunks : ok
    let mut parts = Vec::new();
    for (i, chunk) in data.chunks(5243000).enumerate() {
        let req = ctx
            .owner_auth_req(PutMultipartChunkRequest {
                offer_file_id: offer_file_id.clone(),
                upload_id: upload_id.clone(),
                part_number: i as i32 + 1,
                chunk: chunk.to_owned().into(),
            })
            .await;
        let part = commerce_client
            .put_multipart_chunk(req)
            .await
            .unwrap()
            .into_inner()
            .part
            .unwrap();
        parts.push(part);
    }

    // Complete Multipart Upload : ok
    let req = ctx
        .owner_auth_req(CompleteMultipartUploadRequest {
            offer_file_id: offer_file_id.clone(),
            upload_id: upload_id.clone(),
            parts: parts,
        })
        .await;
    commerce_client
        .complete_multipart_upload(req)
        .await
        .unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert_eq!(offer.files.len(), 1);
    let file = offer.files.first().unwrap();
    assert_eq!(file.total_size_bytes, file.uploaded_size_bytes);

    // Delete File : ok
    let req = ctx
        .owner_auth_req(RemoveFileFromOfferRequest {
            offer_file_id: file.offer_file_id.clone(),
        })
        .await;
    commerce_client.remove_file_from_offer(req).await.unwrap();

    // Delete Offer : ok
    delete_offer(&mut ctx, &mut commerce_client, &offer.offer_id).await;
}
