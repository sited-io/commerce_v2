use tonic::transport::Channel;

use service_apis::sited_io::commerce::v2::commerce_service_client::CommerceServiceClient;
use service_apis::sited_io::commerce::v2::offer_type::{
    self, Digital, OfferTypeKind, Physical,
};
use service_apis::sited_io::commerce::v2::price_type::{
    OneTime, PriceTypeKind, Recurring,
};
use service_apis::sited_io::commerce::v2::{
    offer, CreateOfferRequest, DeleteOfferRequest, GetOfferRequest, Offer,
    OfferType, PriceType,
};

use super::{random_string, TestContext};

pub async fn create_offer(
    ctx: &mut TestContext,
    client: &mut CommerceServiceClient<Channel>,
) -> Offer {
    let req = ctx
        .auth_req(CreateOfferRequest {
            details: Some(offer::Details {
                name: random_string(19),
                description: None,
            }),
            offer_type: Some(OfferType {
                offer_type_kind: Some(offer_type::OfferTypeKind::Digital(
                    offer_type::Digital {},
                )),
            }),
        })
        .await;
    client
        .create_offer(req)
        .await
        .unwrap()
        .into_inner()
        .offer
        .unwrap()
}

pub async fn get_offer(
    client: &mut CommerceServiceClient<Channel>,
    offer_id: &str,
) -> Offer {
    client
        .get_offer(GetOfferRequest {
            offer_id: offer_id.to_owned(),
        })
        .await
        .unwrap()
        .into_inner()
        .offer
        .unwrap()
}

pub async fn delete_offer(
    ctx: &mut TestContext,
    client: &mut CommerceServiceClient<Channel>,
    offer_id: &str,
) {
    let req = ctx
        .auth_req(DeleteOfferRequest {
            offer_id: offer_id.to_owned(),
        })
        .await;
    client.delete_offer(req).await.unwrap();
}

pub fn offer_type_physical() -> OfferType {
    OfferType {
        offer_type_kind: Some(OfferTypeKind::Physical(Physical {})),
    }
}

pub fn offer_type_digital() -> OfferType {
    OfferType {
        offer_type_kind: Some(OfferTypeKind::Digital(Digital {})),
    }
}

pub fn price_type_one_time() -> PriceType {
    PriceType {
        price_type_kind: Some(PriceTypeKind::OneTime(OneTime {})),
    }
}

pub fn price_type_recurring() -> PriceType {
    PriceType {
        price_type_kind: Some(PriceTypeKind::Recurring(Recurring {
            interval: 1,
            interval_count: 14,
            trial_period_days: None,
        })),
    }
}
