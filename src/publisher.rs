use async_nats::client::FlushError;
use async_nats::{Client, ConnectOptions};

use service_apis::sited_io::commerce::v2::{Offer, Shop};

pub struct Publisher {
    client: Client,
}

impl Publisher {
    const OFFER_UPSERT_SUBJECT: &str = "commerce.v2.offer.upsert";
    const OFFER_DELETE_SUBJECT: &str = "commerce.v2.offer.delete";
    const SHOP_UPSERT_SUBJECT: &str = "commerce.v2.shop.upsert";
    const SHOP_DELETE_SUBJECT: &str = "commerce.v2.shop.delete";

    pub async fn init(
        nats_user: String,
        nats_password: String,
        nats_host: String,
    ) -> Self {
        Self {
            client: ConnectOptions::new()
                .user_and_password(nats_user, nats_password)
                .connect(nats_host)
                .await
                .unwrap(),
        }
    }

    pub async fn flush(&self) -> Result<(), FlushError> {
        self.client.flush().await
    }

    pub async fn publish_offer_upsert(&self, offer: Option<&Offer>) {
        if let Some(offer) = offer {
            self.publish(Self::OFFER_UPSERT_SUBJECT, offer).await
        }
    }

    pub async fn publish_offer_delete(&self, offer: Option<&Offer>) {
        if let Some(offer) = offer {
            self.publish(Self::OFFER_DELETE_SUBJECT, offer).await
        }
    }

    pub async fn publish_shop_upsert(&self, shop: Option<&Shop>) {
        if let Some(shop) = shop {
            self.publish(Self::SHOP_UPSERT_SUBJECT, shop).await
        }
    }

    pub async fn publish_shop_delete(&self, shop: Option<&Shop>) {
        if let Some(shop) = shop {
            self.publish(Self::SHOP_DELETE_SUBJECT, shop).await
        }
    }

    async fn publish(
        &self,
        subject: &'static str,
        message: &impl prost::Message,
    ) {
        if let Err(err) = self
            .client
            .publish(subject, message.encode_to_vec().into())
            .await
        {
            tracing::error!(
                "[Publisher]: Error while publishing to {}. {}",
                subject,
                err
            );
        }
    }
}
