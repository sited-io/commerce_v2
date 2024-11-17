use service_apis::sited_io::commerce::v2::offer::Details;
use service_apis::sited_io::commerce::v2::offer_type::{
    Digital, OfferTypeKind, Physical,
};
use service_apis::sited_io::commerce::v2::order_type::{
    OneOff, OrderTypeKind, Subscription,
};
use service_apis::sited_io::commerce::v2::payment_method::{
    PaymentMethodKind, Stripe,
};
use service_apis::sited_io::commerce::v2::price_type::recurring::Interval;
use service_apis::sited_io::commerce::v2::price_type::{
    OneTime, PriceTypeKind, Recurring,
};
use service_apis::sited_io::commerce::v2::stripe_account::{
    Configured, Pending, Status,
};
use service_apis::sited_io::commerce::v2::{
    Offer, OfferFile, OfferImage, OfferPrice, OfferType, Order, OrderType,
    PaymentMethod, PriceType, ShippingRate, Shop, StripeAccount, UserQuota,
};
use service_apis::sited_io::types::country::v1::CountryCode;
use service_apis::sited_io::types::currency::v1::CurrencyCode;

use crate::prisma::{
    offer, offer_details, offer_file, offer_image, offer_price,
    offer_shipping_rate, order, order_type_one_off, order_type_subscription,
    payment_method_stripe, price_recurring, shop, stripe_account,
    stripe_account_status_configured, stripe_account_status_pending,
    user_quota, OfferTypeKey, OrderTypeKey, PaymentMethodKey, PriceTypeKey,
    StripeAccountStatus,
};

impl From<offer::Data> for Offer {
    fn from(value: offer::Data) -> Self {
        Self {
            offer_id: value.offer_id,
            owner: value.owner,
            created_at: value.created_at.timestamp(),
            updated_at: value.updated_at.timestamp(),
            details: value.details.flatten().map(Details::from),
            offer_type: Some(value.offer_type.into()),
            price: value.price.flatten().map(OfferPrice::from),
            shipping_rate: value
                .shipping_rate
                .flatten()
                .map(ShippingRate::from),
            images: value
                .images
                .unwrap_or_default()
                .into_iter()
                .map(OfferImage::from)
                .collect(),
            files: value
                .files
                .unwrap_or_default()
                .into_iter()
                .map(OfferFile::from)
                .collect(),
        }
    }
}

impl From<offer_details::Data> for Details {
    fn from(value: offer_details::Data) -> Self {
        Self {
            name: value.name,
            description: value.description,
        }
    }
}

impl From<Box<offer_details::Data>> for Details {
    fn from(value: Box<offer_details::Data>) -> Self {
        (*value).into()
    }
}

impl From<OfferTypeKind> for OfferTypeKey {
    fn from(value: OfferTypeKind) -> Self {
        match value {
            OfferTypeKind::Physical(_) => Self::Physical,
            OfferTypeKind::Digital(_) => Self::Digital,
        }
    }
}

impl From<OfferTypeKey> for OfferTypeKind {
    fn from(value: OfferTypeKey) -> Self {
        match value {
            OfferTypeKey::Physical => Self::Physical(Physical {}),
            OfferTypeKey::Digital => Self::Digital(Digital {}),
        }
    }
}

impl From<OfferTypeKey> for OfferType {
    fn from(value: OfferTypeKey) -> Self {
        Self {
            offer_type_kind: Some(value.into()),
        }
    }
}

impl From<offer_price::Data> for OfferPrice {
    fn from(value: offer_price::Data) -> Self {
        Self {
            unit_amount: value.unit_amount as u32,
            currency: CurrencyCode::from_str_name(&value.currency)
                .unwrap()
                .into(),
            price_type: Some(build_price_type(
                value.price_type,
                value.price_type_recurring.flatten(),
            )),
        }
    }
}

fn build_price_type(
    price_type_key: PriceTypeKey,
    price_recurring: Option<Box<price_recurring::Data>>,
) -> PriceType {
    match price_type_key {
        PriceTypeKey::OneTime => PriceType {
            price_type_kind: Some(PriceTypeKind::OneTime(OneTime {})),
        },
        PriceTypeKey::Recurring => {
            let price_recurring::Data {
                interval,
                interval_count,
                trial_period_days,
                ..
            } = *price_recurring.unwrap();

            let interval = Interval::from_str_name(&interval).unwrap() as i32;

            PriceType {
                price_type_kind: Some(PriceTypeKind::Recurring(Recurring {
                    interval,
                    interval_count: interval_count as u32,
                    trial_period_days: trial_period_days.map(|t| t as u32),
                })),
            }
        }
    }
}

impl From<Box<offer_price::Data>> for OfferPrice {
    fn from(value: Box<offer_price::Data>) -> Self {
        (*value).into()
    }
}

impl From<PriceTypeKind> for PriceTypeKey {
    fn from(value: PriceTypeKind) -> Self {
        match value {
            PriceTypeKind::OneTime(_) => PriceTypeKey::OneTime,
            PriceTypeKind::Recurring(_) => PriceTypeKey::Recurring,
        }
    }
}

impl From<PriceType> for PriceTypeKey {
    fn from(value: PriceType) -> Self {
        match value.price_type_kind {
            Some(PriceTypeKind::OneTime(_)) => Self::OneTime,
            Some(PriceTypeKind::Recurring(_)) => Self::Recurring,
            None => unreachable!(),
        }
    }
}

impl From<offer_shipping_rate::Data> for ShippingRate {
    fn from(value: offer_shipping_rate::Data) -> Self {
        Self {
            unit_amount: value.unit_amount as u32,
            currency: CurrencyCode::from_str_name(&value.currency)
                .unwrap()
                .into(),
            all_countries: value.all_countries,
            specific_countries: value
                .specific_countries
                .into_iter()
                .map(|c| CountryCode::from_str_name(&c).unwrap().into())
                .collect(),
        }
    }
}

impl From<Box<offer_shipping_rate::Data>> for ShippingRate {
    fn from(value: Box<offer_shipping_rate::Data>) -> Self {
        (*value).into()
    }
}

impl From<offer_image::Data> for OfferImage {
    fn from(value: offer_image::Data) -> Self {
        Self {
            offer_image_id: value.offer_image_id,
            owner: value.owner,
            image_url: value.image_url,
            ordering: value.ordering,
        }
    }
}

impl From<offer_file::Data> for OfferFile {
    fn from(value: offer_file::Data) -> Self {
        Self {
            offer_file_id: value.offer_file_id,
            offer_id: value.offer_id,
            owner: value.owner,
            file_name: value.file_name,
            content_type: value.content_type,
            total_size_bytes: value.total_size_bytes as u64,
            uploaded_size_bytes: value.uploaded_size_bytes as u64,
            ordering: value.ordering,
            file_url: value.file_url,
        }
    }
}

impl From<user_quota::Data> for UserQuota {
    fn from(value: user_quota::Data) -> Self {
        Self {
            user_id: value.user_id,
            max_allowed_size_bytes: value.max_allowed_size_bytes as u64,
            uploaded_size_bytes: value.uploaded_size_bytes as u64,
        }
    }
}

impl From<shop::Data> for Shop {
    fn from(value: shop::Data) -> Self {
        Self {
            shop_id: value.shop_id,
            owner: value.owner,
            website_id: value.website_id,
            offers: value
                .offers
                .unwrap_or_default()
                .into_iter()
                .map(Offer::from)
                .collect(),
        }
    }
}

impl From<order::Data> for Order {
    fn from(value: order::Data) -> Self {
        Self {
            order_id: value.order_id,
            buyer_user_id: value.buyer_user_id,
            offer_id: value.offer_id,
            created_at: value.created_at.timestamp(),
            updated_at: value.updated_at.timestamp(),
            order_type: Some(build_order_type(
                value.order_type,
                value.order_type_one_off.flatten(),
                value.order_type_subscription.flatten(),
            )),
            payment_method: Some(build_payment_method(
                value.payment_method,
                value.payment_method_stripe.flatten(),
            )),
        }
    }
}

fn build_order_type(
    order_type_key: OrderTypeKey,
    order_type_one_off: Option<Box<order_type_one_off::Data>>,
    order_type_subscription: Option<Box<order_type_subscription::Data>>,
) -> OrderType {
    match order_type_key {
        OrderTypeKey::OneOff => {
            let one_off = order_type_one_off.expect("OrderTypeKey == OneOff");
            OrderType {
                order_type_kind: Some(OrderTypeKind::OneOff(OneOff {
                    payed_at: one_off.payed_at.map(|p| p.timestamp()),
                })),
            }
        }
        OrderTypeKey::Subscription => {
            let subscription =
                order_type_subscription.expect("OrderTypeKey == Subscription");
            OrderType {
                order_type_kind: Some(OrderTypeKind::Subscription(
                    Subscription {
                        current_period_start: subscription
                            .current_period_start
                            .timestamp(),
                        current_period_end: subscription
                            .current_period_end
                            .timestamp(),
                        status: subscription.status,
                        payed_at: subscription.payed_at.map(|p| p.timestamp()),
                        payed_until: subscription
                            .payed_untill
                            .map(|p| p.timestamp()),
                        canceled_at: subscription
                            .cancelled_at
                            .map(|c| c.timestamp()),
                        cancel_at: subscription
                            .cancel_at
                            .map(|c| c.timestamp()),
                    },
                )),
            }
        }
    }
}

fn build_payment_method(
    payment_method_key: PaymentMethodKey,
    payment_method_stripe: Option<Box<payment_method_stripe::Data>>,
) -> PaymentMethod {
    match payment_method_key {
        PaymentMethodKey::Stripe => {
            let stripe =
                payment_method_stripe.expect("PaymentMethodKey == Stripe");
            PaymentMethod {
                payment_method_kind: Some(PaymentMethodKind::Stripe(Stripe {
                    subscription_id: stripe.stripe_subscription_id,
                })),
            }
        }
    }
}

impl From<OrderTypeKind> for OrderTypeKey {
    fn from(value: OrderTypeKind) -> Self {
        match value {
            OrderTypeKind::OneOff(_) => OrderTypeKey::OneOff,
            OrderTypeKind::Subscription(_) => OrderTypeKey::Subscription,
        }
    }
}

impl From<OrderType> for OrderTypeKey {
    fn from(value: OrderType) -> Self {
        match value.order_type_kind {
            Some(OrderTypeKind::OneOff(_)) => Self::OneOff,
            Some(OrderTypeKind::Subscription(_)) => Self::Subscription,
            None => unreachable!(),
        }
    }
}

impl From<stripe_account::Data> for StripeAccount {
    fn from(value: stripe_account::Data) -> Self {
        Self {
            stripe_account_id: value.stripe_account_id,
            status: Some(build_stripe_account_status(
                value.status,
                value.status_pending.flatten(),
                value.status_configured.flatten(),
            )),
        }
    }
}

fn build_stripe_account_status(
    status: StripeAccountStatus,
    status_pending: Option<Box<stripe_account_status_pending::Data>>,
    status_configured: Option<Box<stripe_account_status_configured::Data>>,
) -> Status {
    match status {
        StripeAccountStatus::Pending => {
            let status_pending =
                status_pending.expect("StripeAccountStatus == Pending");
            Status::Pending(Pending {
                link: status_pending.link,
            })
        }
        StripeAccountStatus::Configured => {
            let status_configured =
                status_configured.expect("StripeAccountStatus == Configured");
            Status::Configured(Configured {
                charges_enabled: status_configured.charges_enabled,
                details_submitted: status_configured.details_submitted,
            })
        }
    }
}
