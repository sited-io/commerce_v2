use std::collections::HashMap;
use std::str::FromStr;

use stripe::{AccountId, Client, ClientBuilder};
use stripe_billing::subscription::UpdateSubscription;
use stripe_checkout::checkout_session::*;
use stripe_checkout::CheckoutSessionMode;
use stripe_connect::account::{
    CapabilitiesParam, CapabilityParam, CreateAccount, CreateAccountType,
    RetrieveAccount,
};
use stripe_connect::account_link::{CreateAccountLink, CreateAccountLinkType};
use stripe_connect::Account;
use stripe_types::Currency;
use tonic::Status;

use crate::countries::ALL_STRIPE_COUNTRIES;
use crate::prisma::offer_details;
use crate::prisma::offer_price;
use crate::prisma::order;
use crate::prisma::{offer, PriceTypeKey};
use crate::Error;

pub struct StripeService {
    pub client: Client,
    secret_key: String,
}

impl StripeService {
    fn capability_requested() -> Option<CapabilityParam> {
        Some(CapabilityParam {
            requested: Some(true),
        })
    }

    fn shipping_rate_key() -> String {
        String::from("SHIPPING")
    }

    fn with_account(&self, account_id: AccountId) -> Client {
        ClientBuilder::new(self.secret_key.clone())
            .account_id(account_id)
            .build()
            .expect("invalid secret provided")
    }

    pub fn init(secret_key: String) -> Self {
        Self {
            client: Client::new(secret_key.clone()),
            secret_key,
        }
    }

    pub async fn create_account(&self) -> Result<Account, Error> {
        Ok(CreateAccount::new()
            .type_(CreateAccountType::Standard)
            .capabilities(CapabilitiesParam {
                amazon_pay_payments: Self::capability_requested(),
                bank_transfer_payments: Self::capability_requested(),
                ..Default::default()
            })
            .send(&self.client)
            .await?)
    }

    pub async fn create_account_link(
        &self,
        stripe_account_id: &str,
        refresh_url: &str,
        return_url: &str,
    ) -> Result<String, Error> {
        let res = CreateAccountLink::new(
            stripe_account_id,
            CreateAccountLinkType::AccountOnboarding,
        )
        .refresh_url(refresh_url)
        .return_url(return_url)
        .send(&self.client)
        .await?;

        Ok(res.url)
    }

    pub async fn get_account(
        &self,
        stripe_account_id: &str,
    ) -> Result<Account, Error> {
        Ok(RetrieveAccount::new(stripe_account_id)
            .send(&self.client)
            .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_checkout_session(
        &self,
        stripe_account_id: String,
        success_url: String,
        cancel_url: String,
        metadata: HashMap<String, String>,
        application_fee_amount: i64,
        application_fee_percent: f64,
        offer: offer::Data,
        price: offer_price::Data,
    ) -> Result<String, Error> {
        let mut create_checkout_session = CreateCheckoutSession::new()
            .success_url(success_url)
            .cancel_url(cancel_url)
            .metadata(metadata);

        let mode = match price.price_type {
            PriceTypeKey::OneTime => CheckoutSessionMode::Payment,
            PriceTypeKey::Recurring => CheckoutSessionMode::Subscription,
        };
        create_checkout_session = create_checkout_session.mode(mode);

        match price.price_type {
            PriceTypeKey::OneTime => {
                create_checkout_session = create_checkout_session
                    .payment_intent_data(
                        CreateCheckoutSessionPaymentIntentData {
                            application_fee_amount: Some(
                                application_fee_amount,
                            ),
                            ..Default::default()
                        },
                    );
            }
            PriceTypeKey::Recurring => {
                create_checkout_session = create_checkout_session
                    .subscription_data(CreateCheckoutSessionSubscriptionData {
                        application_fee_percent: Some(application_fee_percent),
                        trial_period_days: price
                            .price_type_recurring
                            .clone()
                            .flatten()
                            .and_then(|p| {
                                p.trial_period_days.map(|p| p as u32)
                            }),
                        ..Default::default()
                    });
            }
        }

        if let Some(shipping_rate) = offer.shipping_rate.flatten() {
            let allowed_countries = if shipping_rate.all_countries {
                ALL_STRIPE_COUNTRIES.to_vec()
            } else {
                shipping_rate.specific_countries.iter().map(|c| CreateCheckoutSessionShippingAddressCollectionAllowedCountries::from_str(c).unwrap()).collect()
            };
            create_checkout_session = create_checkout_session
                .shipping_address_collection(
                    CreateCheckoutSessionShippingAddressCollection::new(
                        allowed_countries,
                    ),
                );
            let fixed_amount = CreateCheckoutSessionShippingOptionsShippingRateDataFixedAmount {
                        amount: shipping_rate.unit_amount.into(),
                        currency: Currency::from_str(&shipping_rate.currency.to_lowercase()).unwrap(),
                        currency_options: None,
                    };
            let shipping_rate_data = CreateCheckoutSessionShippingOptionsShippingRateData{
                        display_name: Self::shipping_rate_key(),
                        type_: Some(CreateCheckoutSessionShippingOptionsShippingRateDataType::FixedAmount),
                        fixed_amount: Some(fixed_amount),
                        delivery_estimate: None,
                        metadata: None,
                        tax_behavior: None,
                        tax_code: None
                    };
            create_checkout_session =
                create_checkout_session.shipping_options(vec![
                    CreateCheckoutSessionShippingOptions {
                        shipping_rate: None,
                        shipping_rate_data: Some(shipping_rate_data),
                    },
                ]);
        };

        // Add line items to checkout session
        let offer_details::Data {
            name, description, ..
        } = *offer.details.unwrap().unwrap();

        let product = CreateCheckoutSessionLineItemsPriceDataProductData {
            name,
            description,
            images: offer
                .images
                .map(|is| is.into_iter().map(|i| i.image_url).collect()),
            metadata: None,
            tax_code: None,
        };

        let recurring = match price.price_type {
                    PriceTypeKey::OneTime => None,
                    PriceTypeKey::Recurring => {
                        price.price_type_recurring.flatten().map(|r| {
                            CreateCheckoutSessionLineItemsPriceDataRecurring {
                                interval: CreateCheckoutSessionLineItemsPriceDataRecurringInterval::from_str(
                                    &r.interval.to_lowercase(),
                                ).unwrap(),
                                interval_count: Some(r.interval_count as u64),
                            }
                        })
                    }
                };

        let price_data = CreateCheckoutSessionLineItemsPriceData {
            currency: Currency::from_str(&price.currency.to_lowercase())
                .unwrap(),
            product_data: Some(product),
            unit_amount: Some(i64::from(price.unit_amount)),
            recurring,
            product: None,
            tax_behavior: None,
            unit_amount_decimal: None,
        };

        let adjustable_quantity =
            CreateCheckoutSessionLineItemsAdjustableQuantity {
                enabled: true,
                minimum: Some(1),
                maximum: None,
            };

        let line_items = vec![CreateCheckoutSessionLineItems {
            quantity: Some(1),
            adjustable_quantity: Some(adjustable_quantity),
            price_data: Some(price_data),
            price: None,
            dynamic_tax_rates: None,
            tax_rates: None,
        }];

        let client = self.with_account(stripe_account_id.into());

        let link = create_checkout_session
            .line_items(line_items)
            .send(&client)
            .await?
            .url
            .ok_or_else(|| Status::internal(""))?;

        Ok(link)
    }

    pub async fn update_subscription_period_end(
        &self,
        stripe_account_id: String,
        order: order::Data,
        cancel_at_period_end: bool,
    ) -> Result<(), Error> {
        let stripe_subscription_id = order
            .payment_method_stripe
            .flatten()
            .and_then(|o| o.stripe_subscription_id)
            .ok_or_else(|| {
                Status::failed_precondition(format!(
                    "Could not find payment method stripe for order '{}'",
                    order.order_id
                ))
            })?;

        let client = self.with_account(stripe_account_id.into());

        UpdateSubscription::new(stripe_subscription_id)
            .cancel_at_period_end(cancel_at_period_end)
            .send(&client)
            .await?;

        Ok(())
    }
}
