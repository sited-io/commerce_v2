use std::time::Duration;

use prost::Message;

use service_apis::sited_io::commerce::v2::order_type;
use service_apis::sited_io::commerce::v2::payment_method;
use service_apis::sited_io::commerce::v2::price_type::{
    OneTime, PriceTypeKind,
};
use service_apis::sited_io::commerce::v2::{
    buy_offer_request, buy_offer_response, BuyOfferRequest, CreateShopRequest,
    GetOrderRequest, ListOrdersRequest, OrderType, Payment, PaymentMethod,
    PriceType, PutPriceToOfferRequest,
};
use service_apis::sited_io::websites::v1::WebsiteResponse;

use commerce_v2::common::get_env_var_str;

mod common;

use common::offer::create_offer;

const PAYMENTS_UPSERT_SUBJECT: &str = "payments.payment.upsert";

#[tokio::test]
async fn order_offer_test() {
    let (mut ctx, mut commerce_client) = common::setup().await;

    common::cleanup_offers(&mut ctx, &mut commerce_client).await;

    let nats_client = common::setup_nats_client().await;

    let website_id = get_env_var_str("TEST_INTEGRATION_TEST_WEBSITE_ID");

    let website = WebsiteResponse {
        website_id: website_id.clone(),
        user_id: ctx.owner_user_id(),
        created_at: 1732298141,
        updated_at: 1732298141,
        name: "Integration Test Website".to_string(),
        client_id: "".to_string(),
        customization: None,
        domains: vec![],
        pages: vec![],
    };
    nats_client
        .publish("websites.website.upsert", website.encode_to_vec().into())
        .await
        .unwrap();
    nats_client.flush().await.unwrap();

    // let req = ctx
    //     .owner_auth_req(CreateStripeAccountRequest {
    //         website_id: website_id.clone(),
    //         refresh_url: "http://localhost:8000/stripe/refresh".to_string(),
    //         return_url: "http://localhost:8000/stripe/return".to_string(),
    //     })
    //     .await;
    // let stripe_account = commerce_client
    //     .create_stripe_account(req)
    //     .await
    //     .unwrap()
    //     .into_inner()
    //     .stripe_account
    //     .unwrap();
    // tracing::info!("{:?}", stripe_account);

    let offer = create_offer(&mut ctx, &mut commerce_client).await;
    let req = ctx
        .owner_auth_req(PutPriceToOfferRequest {
            offer_id: offer.offer_id.to_owned(),
            unit_amount: 1400,
            currency: 1,
            price_type: Some(PriceType {
                price_type_kind: Some(PriceTypeKind::OneTime(OneTime {})),
            }),
        })
        .await;
    commerce_client.put_price_to_offer(req).await.unwrap();

    let req = ctx
        .owner_auth_req(CreateShopRequest {
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

    let req = BuyOfferRequest {
        offer_id: offer.offer_id.to_owned(),
        shop_id: shop.shop_id.to_owned(),
        payment_method: Some(buy_offer_request::PaymentMethod::Stripe(
            buy_offer_request::Stripe {
                success_url: "http://localhost:8000/stripe/success".to_string(),
                cancel_url: "http://localhost:8000/stripe/cancel".to_string(),
            },
        )),
    };
    let payment_method = commerce_client
        .buy_offer(req)
        .await
        .unwrap()
        .into_inner()
        .payment_method
        .unwrap();

    let buy_offer_response::PaymentMethod::Stripe(payment_method) =
        payment_method;

    assert!(!payment_method.link.is_empty());

    let req = ctx
        .owner_auth_req(ListOrdersRequest {
            offer_id: Some(offer.offer_id.clone()),
        })
        .await;
    let orders = commerce_client
        .list_orders(req)
        .await
        .unwrap()
        .into_inner()
        .orders;

    assert_eq!(orders.len(), 1);
    let order = orders.first().unwrap();

    let payed_at = 1732554164;
    let order_type = OrderType {
        order_type_kind: Some(order_type::OrderTypeKind::OneOff(
            order_type::OneOff {
                payed_at: Some(payed_at),
            },
        )),
    };

    let payment_method = PaymentMethod {
        payment_method_kind: Some(payment_method::PaymentMethodKind::Stripe(
            payment_method::Stripe {
                subscription_id: None,
            },
        )),
    };

    let payment = Payment {
        order_id: order.order_id.clone(),
        offer_id: offer.offer_id.clone(),
        buyer_user_id: None,
        order_type: Some(order_type),
        payment_method: Some(payment_method),
    };

    nats_client
        .publish(PAYMENTS_UPSERT_SUBJECT, payment.encode_to_vec().into())
        .await
        .unwrap();
    nats_client.flush().await.unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;

    let req = ctx
        .owner_auth_req(GetOrderRequest {
            order_id: order.order_id.clone(),
        })
        .await;
    let order = commerce_client
        .get_order(req)
        .await
        .unwrap()
        .into_inner()
        .order
        .unwrap();

    let order_type::OrderTypeKind::OneOff(one_off) =
        order.order_type.unwrap().order_type_kind.unwrap()
    else {
        panic!("Expected order type to be one off");
    };
    assert_eq!(one_off.payed_at, Some(payed_at));
}
