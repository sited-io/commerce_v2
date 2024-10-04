use prisma_client_rust::or;
use std::sync::Arc;

use service_apis::sited_io::commerce::v2::offer::Details;
use service_apis::sited_io::commerce::v2::{
    Offer, OfferType, OffersFilterField, OffersOrderByField, PriceType, Shop,
};
use service_apis::sited_io::query::v1::Direction;

use crate::prisma::offer::{OrderByParam, WhereParam};
use crate::prisma::read_filters::{
    OfferTypeKeyFilter, StringFilter, StringNullableFilter,
};
use crate::prisma::{
    offer, offer_details, offer_image, offer_price, offer_type, price_type,
    shipping_rate, shop, sub_file, OfferTypeKey, PrismaClient,
};
use crate::Error;

#[derive(Clone)]
pub struct CommerceRepository {
    db: Arc<PrismaClient>,
}

impl CommerceRepository {
    pub fn new(db: Arc<PrismaClient>) -> Self {
        Self { db }
    }

    pub async fn create_offer(
        &self,
        owner: &String,
        details: &Details,
        offer_type: &OfferType,
    ) -> Result<String, Error> {
        let offer = self
            .db
            .offer()
            .create(owner.to_owned(), vec![])
            .exec()
            .await?;

        let Some(offer_type_kind) = offer_type.offer_type_kind else {
            return Err(Error {});
        };

        self.db
            .offer_details()
            .create(
                owner.to_owned(),
                details.name.to_owned(),
                offer::offer_id::equals(offer.offer_id.to_owned()),
                vec![offer_details::description::set(
                    details.description.to_owned(),
                )],
            )
            .exec()
            .await?;

        self.db
            .offer_type()
            .create(
                owner.to_owned(),
                offer_type_kind.into(),
                offer::offer_id::equals(offer.offer_id.clone()),
                vec![],
            )
            .exec()
            .await?;

        Ok(offer.offer_id)
    }

    pub async fn get_offer(
        &self,
        offer_id: &str,
    ) -> Result<Option<Offer>, Error> {
        Ok(self
            .db
            .offer()
            .find_unique(offer::offer_id::equals(offer_id.to_owned()))
            .exec()
            .await?
            .map(Offer::from))
    }

    pub async fn get_extended_offer(
        &self,
        offer_id: &String,
    ) -> Result<Option<Offer>, Error> {
        Ok(self
            .db
            .offer()
            .find_unique(offer::offer_id::equals(offer_id.to_owned()))
            .with(offer::details::fetch())
            .with(offer::offer_type::fetch())
            .with(offer::price::fetch().with(offer_price::price_type::fetch()))
            .with(offer::shipping_rates::fetch(vec![]))
            .with(offer::images::fetch(vec![]))
            .exec()
            .await?
            .map(Offer::from))
    }

    pub async fn list_extended_offers(
        &self,
        owner: Option<&String>,
        shop_id: Option<&String>,
        skip: i64,
        take: i64,
        filter: Option<(OffersFilterField, String)>,
        order_by: Option<(OffersOrderByField, Direction)>,
    ) -> Result<(Vec<Offer>, u32), Error> {
        let mut query = vec![];
        if let Some(owner) = owner {
            query.push(offer::owner::equals(owner.to_owned()));
        }
        if let Some(shop_id) = shop_id {
            query.push(offer::shops::some(vec![shop::shop_id::equals(
                shop_id.to_owned(),
            )]));
        }

        add_offer_filter(&mut query, filter)?;

        let order_by = get_offer_order_by(order_by);

        let offers = self
            .db
            .offer()
            .find_many(query.clone())
            .with(offer::details::fetch())
            .with(offer::offer_type::fetch())
            .with(offer::price::fetch().with(offer_price::price_type::fetch()))
            .with(offer::shipping_rates::fetch(vec![]))
            .with(offer::images::fetch(vec![]))
            .order_by(order_by)
            .skip(skip)
            .take(take)
            .exec()
            .await?;

        let count: u32 =
            self.db.offer().count(query).exec().await?.try_into()?;

        Ok((offers.into_iter().map(Offer::from).collect(), count))
    }

    pub async fn update_offer(
        &self,
        offer_id: &String,
        owner: &String,
        details: Option<&Details>,
        offer_type: Option<&OfferType>,
    ) -> Result<(), Error> {
        if let Some(details) = details {
            self.db
                .offer_details()
                .update_many(
                    vec![
                        offer_details::offer_id::equals(offer_id.clone()),
                        offer_details::owner::equals(owner.to_owned()),
                    ],
                    vec![
                        offer_details::name::set(details.name.clone()),
                        offer_details::description::set(
                            details.description.clone(),
                        ),
                    ],
                )
                .exec()
                .await?;
        }

        if let Some(offer_type) = offer_type {
            if let Some(offer_type_kind) = offer_type.offer_type_kind {
                self.db
                    .offer_type()
                    .upsert(
                        offer_type::offer_id::equals(offer_id.clone()),
                        offer_type::create(
                            owner.to_owned(),
                            offer_type_kind.into(),
                            offer::offer_id::equals(offer_id.to_owned()),
                            vec![],
                        ),
                        vec![],
                    )
                    .exec()
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn delete_offer(
        &self,
        offer_id: &String,
        owner: &String,
    ) -> Result<(), Error> {
        self.db
            .offer()
            .delete_many(vec![
                offer::offer_id::equals(offer_id.to_owned()),
                offer::owner::equals(owner.to_owned()),
            ])
            .exec()
            .await?;
        Ok(())
    }

    pub async fn upsert_price(
        &self,
        offer_id: &String,
        owner: &String,
        unit_amount: u32,
        currency: &str,
        price_type: &PriceType,
    ) -> Result<(), Error> {
        let unit_amount: i32 = unit_amount.try_into()?;

        self.db
            .offer_price()
            .upsert(
                offer_price::offer_id::equals(offer_id.to_owned()),
                offer_price::create(
                    owner.to_owned(),
                    unit_amount,
                    currency.to_owned(),
                    offer::offer_id::equals(offer_id.to_owned()),
                    vec![],
                ),
                vec![
                    offer_price::unit_amount::set(unit_amount),
                    offer_price::currency::set(currency.to_owned()),
                ],
            )
            .exec()
            .await?;

        if let Some(price_type_kind) = price_type.price_type_kind {
            self.db
                .price_type()
                .upsert(
                    price_type::offer_id::equals(offer_id.to_owned()),
                    price_type::create(
                        owner.to_owned(),
                        price_type_kind.into(),
                        offer_price::offer_id::equals(offer_id.to_owned()),
                        vec![],
                    ),
                    vec![price_type::price_type_key::set(
                        price_type_kind.into(),
                    )],
                )
                .exec()
                .await?;
        }

        Ok(())
    }

    pub async fn delete_offer_price(
        &self,
        offer_id: &String,
        owner: &String,
    ) -> Result<(), Error> {
        self.db
            .offer_price()
            .delete_many(vec![
                offer_price::offer_id::equals(offer_id.to_owned()),
                offer_price::owner::equals(owner.to_owned()),
            ])
            .exec()
            .await?;
        Ok(())
    }

    pub async fn create_shipping_rate(
        &self,
        offer_id: &String,
        owner: &String,
        unit_amount: u32,
        currency: &str,
        all_countries: bool,
        specific_countries: &Vec<String>,
    ) -> Result<(), Error> {
        let unit_amount: i32 = unit_amount.try_into()?;

        self.db
            .shipping_rate()
            .create(
                owner.to_owned(),
                unit_amount,
                currency.to_owned(),
                all_countries,
                offer::offer_id::equals(offer_id.to_owned()),
                vec![shipping_rate::specific_countries::set(
                    specific_countries.to_owned(),
                )],
            )
            .exec()
            .await?;

        Ok(())
    }

    pub async fn delete_shipping_rate(
        &self,
        shipping_rate_id: &String,
        owner: &String,
    ) -> Result<(), Error> {
        self.db
            .shipping_rate()
            .delete_many(vec![
                shipping_rate::shipping_rate_id::equals(
                    shipping_rate_id.to_owned(),
                ),
                shipping_rate::owner::equals(owner.to_owned()),
            ])
            .exec()
            .await?;

        Ok(())
    }

    pub async fn create_offer_image(
        &self,
        offer_id: &String,
        file_id: &String,
        owner: &String,
        ordering: i32,
    ) -> Result<(), Error> {
        self.db
            .offer_image()
            .create(
                owner.to_owned(),
                ordering,
                sub_file::file_id::equals(file_id.to_owned()),
                offer::offer_id::equals(offer_id.to_owned()),
                vec![],
            )
            .exec()
            .await?;

        Ok(())
    }

    pub async fn delete_offer_image(
        &self,
        offer_image_id: &String,
        owner: &String,
    ) -> Result<(), Error> {
        self.db
            .offer_image()
            .delete_many(vec![
                offer_image::offer_image_id::equals(offer_image_id.to_owned()),
                offer_image::owner::equals(owner.to_owned()),
            ])
            .exec()
            .await?;

        Ok(())
    }

    pub async fn create_shop(
        &self,
        owner: &String,
        website_id: &String,
    ) -> Result<Shop, Error> {
        Ok(self
            .db
            .shop()
            .create(owner.to_owned(), website_id.to_owned(), vec![])
            .exec()
            .await?
            .into())
    }

    pub async fn get_shop(
        &self,
        shop_id: &String,
    ) -> Result<Option<Shop>, Error> {
        Ok(self
            .db
            .shop()
            .find_unique(shop::shop_id::equals(shop_id.to_owned()))
            .exec()
            .await?
            .map(Shop::from))
    }

    pub async fn delete_shop(
        &self,
        shop_id: &String,
        owner: &String,
    ) -> Result<(), Error> {
        self.db
            .shop()
            .delete_many(vec![
                shop::shop_id::equals(shop_id.to_owned()),
                shop::owner::equals(owner.to_owned()),
            ])
            .exec()
            .await?;

        Ok(())
    }

    pub async fn add_offer_to_shop(
        &self,
        offer_id: &String,
        shop_id: &String,
    ) -> Result<(), Error> {
        self.db
            .offer()
            .update(
                offer::offer_id::equals(offer_id.to_owned()),
                vec![offer::shops::connect(vec![shop::shop_id::equals(
                    shop_id.to_owned(),
                )])],
            )
            .exec()
            .await?;
        Ok(())
    }

    pub async fn remove_offer_from_shop(
        &self,
        offer_id: &String,
        shop_id: &String,
    ) -> Result<(), Error> {
        self.db
            .offer()
            .update(
                offer::offer_id::equals(offer_id.to_owned()),
                vec![offer::shops::disconnect(vec![shop::shop_id::equals(
                    shop_id.to_owned(),
                )])],
            )
            .exec()
            .await?;
        Ok(())
    }
}

fn add_offer_filter(
    query: &mut Vec<WhereParam>,
    filter: Option<(OffersFilterField, String)>,
) -> Result<(), Error> {
    if let Some((field, value)) = filter {
        match field {
            OffersFilterField::Unspecified => {}
            OffersFilterField::Name => {
                query.push(offer::details::is(vec![
                    offer_details::WhereParam::Name(StringFilter::Contains(
                        value,
                    )),
                ]));
            }
            OffersFilterField::Description => {
                query.push(offer::details::is(vec![
                    offer_details::WhereParam::Description(
                        StringNullableFilter::Contains(value),
                    ),
                ]));
            }
            OffersFilterField::NameAndDescription => {
                query.push(offer::details::is(vec![or![
                    offer_details::WhereParam::Name(StringFilter::Contains(
                        value.clone()
                    ),),
                    offer_details::WhereParam::Description(
                        StringNullableFilter::Contains(value)
                    )
                ]]));
            }
            OffersFilterField::Type => {
                let key = if value == OfferTypeKey::Physical.to_string() {
                    OfferTypeKey::Physical
                } else if value == OfferTypeKey::Digital.to_string() {
                    OfferTypeKey::Digital
                } else {
                    return Err(Error {});
                };
                query.push(offer::offer_type::is(vec![
                    offer_type::WhereParam::OfferTypeKey(
                        OfferTypeKeyFilter::Equals(key),
                    ),
                ]));
            }
        }
    };

    Ok(())
}

fn get_offer_order_by(
    order_by: Option<(OffersOrderByField, Direction)>,
) -> OrderByParam {
    use prisma_client_rust::Direction::*;

    if let Some(order_by) = order_by {
        let direction = match order_by.1 {
            Direction::Unspecified | Direction::Asc => Asc,
            Direction::Desc => Desc,
        };

        match order_by.0 {
            OffersOrderByField::Unspecified | OffersOrderByField::CreatedAt => {
                OrderByParam::CreatedAt(direction)
            }
            OffersOrderByField::UpdatedAt => OrderByParam::UpdatedAt(direction),
        }
    } else {
        OrderByParam::CreatedAt(Desc)
    }
}
