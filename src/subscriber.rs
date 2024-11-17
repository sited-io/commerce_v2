use futures::StreamExt;
use prost::Message;
use service_apis::sited_io::websites::v1::WebsiteResponse;

use crate::{CommerceRepository, Error};

pub struct Subscriber {
    nats_client: async_nats::Client,
    repository: CommerceRepository,
}

impl Subscriber {
    pub fn new(
        nats_client: async_nats::Client,
        repository: CommerceRepository,
    ) -> Self {
        Self {
            nats_client,
            repository,
        }
    }

    pub async fn subscribe(&self) {
        let mut subscriber = self
            .nats_client
            .queue_subscribe(
                "websites.website.>",
                "websites.website".to_string(),
            )
            .await
            .unwrap();

        while let Some(message) = subscriber.next().await {
            let action: &str =
                message.subject.split('.').last().unwrap_or_default();

            let Ok(website) = WebsiteResponse::decode(message.payload) else {
                tracing::error!("[WebsiteSubscriber::subscribe]: could not decode message as WebsiteResponse");
                return;
            };

            if let Err(err) = if action == "upsert" {
                self.repository
                    .upsert_sub_website(&website.website_id, &website.user_id)
                    .await
            } else if action == "delete" {
                self.repository
                    .delete_sub_website(&website.website_id)
                    .await
            } else {
                Err(Error::from(format!(
                    "[WebsiteSubscriber::subscribe]: Unkonwn action {}",
                    action
                )))
            } {
                tracing::error!("[Subscriber::subscrbe] {:?}", err)
            }
        }
    }
}
