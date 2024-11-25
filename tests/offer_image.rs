use service_apis::sited_io::commerce::v2::{
    AddImageToOfferRequest, RemoveImageFromOfferRequest,
};

use common::fixtures::IMAGE_DATA;
use common::offer::{create_offer, delete_offer, get_offer};

mod common;

#[tokio::test]
async fn offer_image_test() {
    let (mut ctx, mut commerce_client) = common::setup().await;

    common::cleanup_offers(&mut ctx, &mut commerce_client).await;

    // Create Offer : ok
    let offer = create_offer(&mut ctx, &mut commerce_client).await;

    // Check Offer no images : ok
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.images.is_empty());

    // Add Image to Offer : ok
    let req = ctx
        .owner_auth_req(AddImageToOfferRequest {
            offer_id: offer.offer_id.clone(),
            data: IMAGE_DATA.into(),
            ordering: 1,
        })
        .await;
    commerce_client.add_image_to_offer(req).await.unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert_eq!(offer.images.len(), 1);
    let image = offer.images.first().unwrap();

    // TODO: Check image ordering

    // Remove Image from Offer : ok
    let req = ctx
        .owner_auth_req(RemoveImageFromOfferRequest {
            offer_image_id: image.offer_image_id.clone(),
            offer_id: offer.offer_id.clone(),
        })
        .await;
    commerce_client.remove_image_from_offer(req).await.unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.images.is_empty());

    // Delete Offer : ok
    delete_offer(&mut ctx, &mut commerce_client, &offer.offer_id).await;
}
