use service_apis::sited_io::commerce::v2::{
    PutPriceToOfferRequest, RemovePriceFromOfferRequest,
};

mod common;

use common::offer::{
    create_offer, delete_offer, get_offer, price_type_one_time,
    price_type_recurring,
};

#[tokio::test]
async fn offer_price_test() {
    let (mut ctx, mut commerce_client) = common::setup().await;

    common::cleanup_offers(&mut ctx, &mut commerce_client).await;

    // Create Offer : ok
    let offer = create_offer(&mut ctx, &mut commerce_client).await;

    // Check Offer no price : ok
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.price.is_none());

    // Put Price : missing inputs : nok
    assert!(commerce_client
        .put_price_to_offer(PutPriceToOfferRequest {
            offer_id: offer.offer_id.clone(),
            unit_amount: 1000,
            currency: 0,
            price_type: None,
        })
        .await
        .is_err());
    assert!(commerce_client
        .put_price_to_offer(PutPriceToOfferRequest {
            offer_id: offer.offer_id.clone(),
            unit_amount: 1000,
            currency: 1,
            price_type: None,
        })
        .await
        .is_err());
    assert!(commerce_client
        .put_price_to_offer(PutPriceToOfferRequest {
            offer_id: offer.offer_id.clone(),
            unit_amount: 1000,
            currency: 0,
            price_type: Some(price_type_one_time()),
        })
        .await
        .is_err());

    // Put Price : ok
    let req = ctx
        .auth_req(PutPriceToOfferRequest {
            offer_id: offer.offer_id.clone(),
            unit_amount: 1450,
            currency: 1,
            price_type: Some(price_type_one_time()),
        })
        .await;
    commerce_client.put_price_to_offer(req).await.unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.price.is_some());
    assert_eq!(offer.price.unwrap().unit_amount, 1450);

    // Put Price : update : ok
    let req = ctx
        .auth_req(PutPriceToOfferRequest {
            offer_id: offer.offer_id.clone(),
            unit_amount: 700,
            currency: 1,
            price_type: Some(price_type_recurring()),
        })
        .await;
    commerce_client.put_price_to_offer(req).await.unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.price.is_some());
    let price = offer.price.unwrap();
    assert_eq!(price.unit_amount, 700);
    assert_eq!(price.price_type.unwrap(), price_type_recurring());

    // Remove Price : ok
    let req = ctx
        .auth_req(RemovePriceFromOfferRequest {
            offer_id: offer.offer_id.clone(),
        })
        .await;
    commerce_client.remove_price_from_offer(req).await.unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.price.is_none());

    // Put Price : ok
    let req = ctx
        .auth_req(PutPriceToOfferRequest {
            offer_id: offer.offer_id.clone(),
            unit_amount: 700,
            currency: 1,
            price_type: Some(price_type_recurring()),
        })
        .await;
    commerce_client.put_price_to_offer(req).await.unwrap();
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert!(offer.price.is_some());

    // Remove Offer : ok
    delete_offer(&mut ctx, &mut commerce_client, &offer.offer_id).await;
}
