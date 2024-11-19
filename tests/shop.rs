use common::offer::{create_offer, delete_offer};
use service_apis::sited_io::commerce::v2::commerce_service_client::CommerceServiceClient;
use service_apis::sited_io::commerce::v2::{
    AddOfferToShopRequest, CreateShopRequest, DeleteShopRequest,
    GetShopRequest, RemoveOfferFromShopRequest, Shop,
};
use tonic::transport::Channel;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn shop_test() {
    let (mut ctx, mut commerce_client) = common::setup().await;

    common::cleanup_offers(&mut ctx, &mut commerce_client).await;

    // Create Shop : ok
    let website_id = Uuid::new_v4().to_string();
    let req = ctx
        .auth_req(CreateShopRequest {
            website_id: website_id.clone(),
        })
        .await;
    let shop = commerce_client
        .create_shop(req)
        .await
        .unwrap()
        .into_inner()
        .shop
        .unwrap();

    // Get Shop : ok
    let shop = commerce_client
        .get_shop(GetShopRequest {
            shop_id: shop.shop_id.clone(),
        })
        .await
        .unwrap()
        .into_inner()
        .shop
        .unwrap();
    assert_eq!(shop.website_id, website_id);
    assert!(shop.offers.is_empty());

    // Add offer to shop : ok
    let offer = create_offer(&mut ctx, &mut commerce_client).await;
    let req = ctx
        .auth_req(AddOfferToShopRequest {
            offer_id: offer.offer_id.clone(),
            shop_id: shop.shop_id.clone(),
        })
        .await;
    commerce_client.add_offer_to_shop(req).await.unwrap();
    let shop = get_shop(&mut commerce_client, &shop.shop_id).await;
    assert_eq!(shop.offers.len(), 1);

    // Remove offer from shop : ok
    let req = ctx
        .auth_req(RemoveOfferFromShopRequest {
            offer_id: offer.offer_id.clone(),
            shop_id: shop.shop_id.clone(),
        })
        .await;
    commerce_client.remove_offer_from_shop(req).await.unwrap();
    let shop = get_shop(&mut commerce_client, &shop.shop_id).await;
    assert!(shop.offers.is_empty());

    delete_offer(&mut ctx, &mut commerce_client, &offer.offer_id).await;

    // Delete Shop
    let req = ctx
        .auth_req(DeleteShopRequest {
            shop_id: shop.shop_id.clone(),
        })
        .await;
    commerce_client.delete_shop(req).await.unwrap();
}

async fn get_shop(
    client: &mut CommerceServiceClient<Channel>,
    shop_id: &String,
) -> Shop {
    client
        .get_shop(GetShopRequest {
            shop_id: shop_id.to_owned(),
        })
        .await
        .unwrap()
        .into_inner()
        .shop
        .unwrap()
}
