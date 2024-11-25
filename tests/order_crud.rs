use service_apis::sited_io::commerce::v2::GetOrderRequest;

mod common;

#[tokio::test]
async fn order_crud_test() {
    let (mut ctx, mut commerce_client) = common::setup().await;

    let req = ctx
        .owner_auth_req(GetOrderRequest {
            order_id: "e3cdbd05-688d-4f7f-99cb-3a2397057feb".to_string(),
        })
        .await;
    let order = commerce_client
        .get_order(req)
        .await
        .unwrap()
        .into_inner()
        .order
        .unwrap();

    tracing::info!("{:?}", order);
}
