use crate::api::sited_io::commerce::v2::offer::Details;
use crate::api::sited_io::commerce::v2::offer_type::{
    Digital, OfferTypeKind, Physical,
};
use crate::api::sited_io::commerce::v2::price_type::recurring::Interval;
use crate::api::sited_io::commerce::v2::price_type::{
    OneTime, PriceTypeKind, Recurring,
};
use crate::api::sited_io::commerce::v2::{
    Offer, OfferImage, OfferPrice, OfferType, PriceType, ShippingRate, Shop,
};
use crate::api::sited_io::country::v1::CountryCode;
use crate::api::sited_io::price::v1::{CurrencyCode, Price};

use crate::prisma::{
    offer, offer_details, offer_image, offer_price, offer_type, price_type,
    shipping_rate, shop, OfferTypeKey, PriceTypeKey,
};

impl From<offer::Data> for Offer {
    fn from(value: offer::Data) -> Self {
        Self {
            offer_id: value.offer_id,
            owner: value.owner,
            created_at: value.created_at.timestamp(),
            updated_at: value.updated_at.timestamp(),
            details: value.details.flatten().map(Details::from),
            offer_type: value.offer_type.flatten().map(OfferType::from),
            price: value.price.flatten().map(OfferPrice::from),
            shipping_rates: value
                .shipping_rates
                .unwrap_or_default()
                .into_iter()
                .map(ShippingRate::from)
                .collect(),
            images: value
                .images
                .unwrap_or_default()
                .into_iter()
                .map(OfferImage::from)
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

impl From<offer_type::Data> for OfferType {
    fn from(value: offer_type::Data) -> Self {
        match value.offer_type_key {
            OfferTypeKey::Physical => Self {
                offer_type_kind: Some(OfferTypeKind::Physical(Physical {})),
            },
            OfferTypeKey::Digital => Self {
                offer_type_kind: Some(OfferTypeKind::Digital(Digital {})),
            },
        }
    }
}

impl From<Box<offer_type::Data>> for OfferType {
    fn from(value: Box<offer_type::Data>) -> Self {
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

impl From<offer_price::Data> for OfferPrice {
    fn from(value: offer_price::Data) -> Self {
        Self {
            price: Some(Price {
                unit_amount: value.unit_amount.try_into().unwrap(),
                currency_code: CurrencyCode::from_str_name(&value.currency)
                    .unwrap()
                    .into(),
            }),
            price_type: value.price_type.flatten().map(|o| (*o).into()),
        }
    }
}

impl From<Box<offer_price::Data>> for OfferPrice {
    fn from(value: Box<offer_price::Data>) -> Self {
        (*value).into()
    }
}

impl From<price_type::Data> for PriceType {
    fn from(value: price_type::Data) -> Self {
        match value.price_type_key {
            PriceTypeKey::OneTime => Self {
                price_type_kind: Some(PriceTypeKind::OneTime(OneTime {})),
            },
            PriceTypeKey::Recurring => Self {
                price_type_kind: Some(PriceTypeKind::Recurring(Recurring {
                    interval: Interval::from_str_name(
                        &value.recurring_interval.unwrap(),
                    )
                    .unwrap()
                    .into(),
                    interval_count: value
                        .recurring_interval_count
                        .unwrap()
                        .try_into()
                        .unwrap(),
                    trial_period_days: value
                        .recurring_trial_period_days
                        .and_then(|i| i.try_into().ok()),
                })),
            },
        }
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

impl From<shipping_rate::Data> for ShippingRate {
    fn from(value: shipping_rate::Data) -> Self {
        Self {
            shipping_rate_id: value.shipping_rate_id,
            owner: value.owner,
            price: Some(Price {
                unit_amount: value.unit_amount.try_into().unwrap(),
                currency_code: CurrencyCode::from_str_name(&value.currency)
                    .unwrap()
                    .into(),
            }),
            all_countries: value.all_countries,
            specific_countries: value
                .specific_countries
                .into_iter()
                .map(|c| CountryCode::from_str_name(&c).unwrap().into())
                .collect(),
        }
    }
}

impl From<offer_image::Data> for OfferImage {
    fn from(value: offer_image::Data) -> Self {
        Self {
            offer_image_id: value.offer_image_id,
            owner: value.owner,
            image_url: value.file.and_then(|f| f.file_url).unwrap_or_default(),
            ordering: value.ordering,
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
