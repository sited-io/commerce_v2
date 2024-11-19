use service_apis::sited_io::commerce::v2::{
    PutShippingRateToOfferRequest, RemoveShippingRateFromOfferRequest,
};

mod common;

use common::offer::{create_offer, delete_offer, get_offer};

#[tokio::test]
async fn offer_shipping_rate_test() {
    let (mut ctx, mut commerce_client) = common::setup().await;

    common::cleanup_offers(&mut ctx, &mut commerce_client).await;

    // Create Offer : ok
    let offer = create_offer(&mut ctx, &mut commerce_client).await;

    // Check Offer no shipping rate : ok
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.shipping_rate.is_none());

    // Put Shipping Rate : ok
    let req = ctx
        .auth_req(PutShippingRateToOfferRequest {
            offer_id: offer.offer_id.clone(),
            unit_amount: 1450,
            currency: 1,
            all_countries: true,
            specific_countries: vec![],
        })
        .await;
    commerce_client
        .put_shipping_rate_to_offer(req)
        .await
        .unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.shipping_rate.is_some());
    assert_eq!(offer.shipping_rate.unwrap().unit_amount, 1450);

    // Put Shipping Rate : update : ok
    let req = ctx
        .auth_req(PutShippingRateToOfferRequest {
            offer_id: offer.offer_id.clone(),
            unit_amount: 700,
            currency: 1,
            all_countries: true,
            specific_countries: vec![],
        })
        .await;
    commerce_client
        .put_shipping_rate_to_offer(req)
        .await
        .unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.shipping_rate.is_some());
    let shipping_rate = offer.shipping_rate.unwrap();
    assert_eq!(shipping_rate.all_countries, true);
    assert_eq!(shipping_rate.currency, 1);
    assert_eq!(shipping_rate.unit_amount, 700);

    // Remove Shipping Rate : ok
    let req = ctx
        .auth_req(RemoveShippingRateFromOfferRequest {
            offer_id: offer.offer_id.clone(),
        })
        .await;
    commerce_client
        .remove_shipping_rate_from_offer(req)
        .await
        .unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.shipping_rate.is_none());

    // Put Shipping Rate : ok
    let req = ctx
        .auth_req(PutShippingRateToOfferRequest {
            offer_id: offer.offer_id.clone(),
            unit_amount: 700,
            currency: 1,
            all_countries: true,
            specific_countries: vec![],
        })
        .await;
    commerce_client
        .put_shipping_rate_to_offer(req)
        .await
        .unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.shipping_rate.is_some());

    // Delete Offer : ok
    delete_offer(&mut ctx, &mut commerce_client, &offer.offer_id).await;
}
