use service_apis::sited_io::commerce::v2::{
    offer, offer_type, CreateOfferRequest, DeleteOfferRequest,
    ListOffersRequest, OfferType, UpdateOfferRequest,
};

mod common;

use common::offer::get_offer;
use common::random_string;

#[tokio::test]
async fn test_offers() {
    let (mut ctx, mut commerce_client) = common::setup().await;

    common::cleanup_offers(&mut ctx, &mut commerce_client).await;

    // List Offers : empty : ok
    let res = commerce_client
        .list_offers(ListOffersRequest {
            owner: Some(ctx.owner_user_id()),
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    assert!(res.offers.is_empty());
    assert!(res
        .pagination
        .is_some_and(|p| p.page == 1 && p.total_elements == 0));

    // Create Offer : ok
    let offer_1_name = random_string(8);
    let req = ctx
        .owner_auth_req(CreateOfferRequest {
            details: Some(offer::Details {
                name: offer_1_name.clone(),
                description: None,
            }),
            offer_type: Some(OfferType {
                offer_type_kind: Some(offer_type::OfferTypeKind::Digital(
                    offer_type::Digital {},
                )),
            }),
        })
        .await;
    let offer = commerce_client
        .create_offer(req)
        .await
        .unwrap()
        .into_inner()
        .offer
        .unwrap();
    let details = offer.details.unwrap();
    assert_eq!(details.name, offer_1_name);
    let offer = get_offer(&mut commerce_client, &offer.offer_id).await;
    assert_eq!(offer.details.unwrap().name, offer_1_name.to_string());

    // List Offers : ok
    let res = commerce_client
        .list_offers(ListOffersRequest {
            owner: Some(ctx.owner_user_id()),
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    let offers = res.offers;
    let pagination = res.pagination.unwrap();
    assert_eq!(offers.len(), 1);
    assert_eq!(pagination.page, 1);
    assert_eq!(pagination.total_elements, 1);

    // Update Offer : ok
    let offer_1_name = random_string(8);
    let req = ctx
        .owner_auth_req(UpdateOfferRequest {
            offer_id: offer.offer_id,
            details: Some(offer::Details {
                name: offer_1_name.clone(),
                description: None,
            }),
            offer_type: None,
        })
        .await;
    let offer = commerce_client
        .update_offer(req)
        .await
        .unwrap()
        .into_inner()
        .offer
        .unwrap();
    let details = offer.details.unwrap();
    assert_eq!(details.name, offer_1_name);

    // Delete Offer : ok
    let req = ctx
        .owner_auth_req(DeleteOfferRequest {
            offer_id: offer.offer_id,
        })
        .await;
    commerce_client.delete_offer(req).await.unwrap();
}
