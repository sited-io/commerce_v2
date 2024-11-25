use futures::StreamExt;
use prost::Message;

use service_apis::sited_io::commerce::v2::order_type::{self, OrderTypeKind};
use service_apis::sited_io::commerce::v2::payment_method::PaymentMethodKind;
use service_apis::sited_io::commerce::v2::Payment;

use crate::common::datetime;
use crate::prisma::PaymentMethodKey;
use crate::CommerceRepository;

pub struct PaymentSubscriber {
    nats_client: async_nats::Client,
    repository: CommerceRepository,
}

impl PaymentSubscriber {
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
                "payments.payment.>",
                "commerce_v2.payment".to_string(),
            )
            .await
            .unwrap();

        while let Some(message) = subscriber.next().await {
            let action: &str =
                message.subject.split('.').last().unwrap_or_default();

            if action != "upsert" {
                tracing::error!(
                    "[PaymentSubscriber::subscribe]: Unknown action {}",
                    action
                );
                return;
            }

            let Ok(payment) = Payment::decode(message.payload) else {
                tracing::error!("[PaymentSubscriber::subscribe]: could not decode message as Payment");
                return;
            };

            tracing::info!(
                "[PaymentSubscriber::subscribe]: Got payment message {:?}",
                payment
            );

            let Ok(Some(order)) =
                self.repository.get_order(&payment.order_id).await
            else {
                tracing::error!(
                    "[PaymentSubscriber::subscribe]: could not find order '{}'",
                    payment.order_id
                );
                return;
            };

            if let Some(payment_method) = payment.payment_method {
                if let Some(PaymentMethodKind::Stripe(stripe)) =
                    payment_method.payment_method_kind
                {
                    if order.payment_method != PaymentMethodKey::Stripe {
                        tracing::error!(
                            "[PaymentSubscriber::subscribe]: payment was for stripe but order was not. order_id: '{}'",
                            order.order_id
                        )
                    }

                    if let None = order.payment_method_stripe.flatten() {
                        if let Err(err) = self
                            .repository
                            .upsert_order_payment_method(
                                &order.order_id,
                                stripe.subscription_id.as_ref(),
                            )
                            .await
                        {
                            tracing::error!("[PaymentSubscriber::subscribe]: upsert_order_payment_method. Error: '{}'", err);
                        }
                    }
                }
            }

            if let Some(order_type) = payment.order_type {
                match order_type.order_type_kind {
                    Some(OrderTypeKind::OneOff(one_off)) => {
                        if let Err(err) = self
                            .repository
                            .upsert_order_type_one_off(
                                &order.order_id,
                                one_off
                                    .payed_at
                                    .map(datetime::ts_to_datetime_fixed),
                            )
                            .await
                        {
                            tracing::error!("[PaymentSubscriber::subscribe]: upsert_order_type_one_off. Error: '{}'", err)
                        }
                    }
                    Some(OrderTypeKind::Subscription(
                        order_type::Subscription {
                            current_period_start,
                            current_period_end,
                            status,
                            payed_at,
                            payed_until,
                            canceled_at,
                            cancel_at,
                        },
                    )) => {
                        if let Err(err) = self
                            .repository
                            .upsert_order_type_subscription(
                                &order.order_id,
                                datetime::ts_to_datetime_fixed(
                                    current_period_start,
                                ),
                                datetime::ts_to_datetime_fixed(
                                    current_period_end,
                                ),
                                &status,
                                payed_at.map(datetime::ts_to_datetime_fixed),
                                payed_until.map(datetime::ts_to_datetime_fixed),
                                canceled_at.map(datetime::ts_to_datetime_fixed),
                                cancel_at.map(datetime::ts_to_datetime_fixed),
                            )
                            .await
                        {
                            tracing::error!("[PaymentSubscriber::subscribe]: upsert_order_type_one_off. Error: '{}'", err)
                        }
                    }
                    None => {
                        tracing::error!("[PaymentSubscriber::subscribe]: payment had no order_type");
                        return;
                    }
                }
            }
        }
    }
}
