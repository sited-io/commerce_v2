use service_apis::sited_io::commerce::v2::{
    AddFileToOfferRequest, DownloadFileRequest, RemoveFileFromOfferRequest,
    UpdateFileOrderingRequest,
};

mod common;
use common::offer::{create_offer, delete_offer, get_offer};

#[tokio::test]
async fn offer_file_test() {
    let (mut ctx, mut commerce_client) = common::setup().await;

    common::cleanup_offers(&mut ctx, &mut commerce_client).await;

    // Create Offer : ok
    let offer = create_offer(&mut ctx, &mut commerce_client).await;

    // Check Offer : empty files : ok
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.files.is_empty());

    // Add File to Offer : ok
    let first_file_name = common::random_string(8);
    let req = ctx
        .owner_auth_req(AddFileToOfferRequest {
            file_name: first_file_name.clone(),
            offer_id: offer.offer_id.clone(),
            content: common::fixtures::IMAGE_DATA.into(),
            content_type: Some("image/jpeg".to_string()),
            ordering: Some(1),
        })
        .await;
    commerce_client.add_file_to_offer(req).await.unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert_eq!(offer.files.len(), 1);

    // Add Second File : ok
    let second_file_name = common::random_string(10);
    let req = ctx
        .owner_auth_req(AddFileToOfferRequest {
            file_name: second_file_name.clone(),
            offer_id: offer.offer_id.clone(),
            content: common::fixtures::IMAGE_DATA.into(),
            content_type: Some("image/jpeg".to_string()),
            ordering: Some(2),
        })
        .await;
    commerce_client.add_file_to_offer(req).await.unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert_eq!(offer.files.len(), 2);
    let first_file = offer.files.get(0).unwrap();
    let second_file = offer.files.get(1).unwrap();
    assert_eq!(first_file.file_name, first_file_name);
    assert_eq!(second_file.file_name, second_file_name);

    // Change Order : ok
    let req = ctx
        .owner_auth_req(UpdateFileOrderingRequest {
            offer_file_id: first_file.offer_file_id.clone(),
            ordering: 3,
        })
        .await;
    commerce_client.update_file_ordering(req).await.unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert_eq!(offer.files.len(), 2);
    let first_file = offer.files.get(0).unwrap();
    let second_file = offer.files.get(1).unwrap();
    assert_eq!(first_file.file_name, second_file_name);
    assert_eq!(second_file.file_name, first_file_name);

    // Download File : ok
    let req = ctx
        .owner_auth_req(DownloadFileRequest {
            offer_file_id: first_file.offer_file_id.clone(),
        })
        .await;
    let download_url = commerce_client
        .download_file(req)
        .await
        .unwrap()
        .into_inner()
        .download_url;
    let res = reqwest::get(download_url)
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(res.to_vec(), common::fixtures::IMAGE_DATA.to_vec());

    // Delete Files : ok
    let req = ctx
        .owner_auth_req(RemoveFileFromOfferRequest {
            offer_file_id: first_file.offer_file_id.clone(),
        })
        .await;
    commerce_client.remove_file_from_offer(req).await.unwrap();
    let req = ctx
        .owner_auth_req(RemoveFileFromOfferRequest {
            offer_file_id: second_file.offer_file_id.clone(),
        })
        .await;
    commerce_client.remove_file_from_offer(req).await.unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.files.is_empty());

    // Delete Offer : ok
    delete_offer(&mut ctx, &mut commerce_client, &offer.offer_id).await;
}
