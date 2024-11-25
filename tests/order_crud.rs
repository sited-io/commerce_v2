use service_apis::sited_io::commerce::v2::ListOrdersRequest;

mod common;

#[tokio::test]
async fn order_crud_test() {
    let (mut ctx, mut commerce_client) = common::setup().await;

    let req = ctx
        .owner_auth_req(ListOrdersRequest { offer_id: None })
        .await;
    let orders = commerce_client
        .list_orders(req)
        .await
        .unwrap()
        .into_inner()
        .orders;

    tracing::info!("{:?}", orders);
}
