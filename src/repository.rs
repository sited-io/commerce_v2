use prisma_client_rust::or;
use std::future::Future;
use std::sync::Arc;
use uuid::Uuid;

use service_apis::sited_io::commerce::v2::offer::Details;
use service_apis::sited_io::commerce::v2::offer_type::OfferTypeKind;
use service_apis::sited_io::commerce::v2::price_type::PriceTypeKind;
use service_apis::sited_io::commerce::v2::{
    OfferType, OffersFilterField, OffersOrderByField, PriceType,
};
use service_apis::sited_io::types::query::v1::Direction;

use crate::prisma::offer::{OrderByParam, WhereParam};
use crate::prisma::read_filters::{StringFilter, StringNullableFilter};
use crate::prisma::{
    offer, offer_details, offer_file, offer_image, offer_price,
    offer_shipping_rate, order, price_recurring, shop, stripe_account,
    stripe_account_status_pending, sub_webiste, user_quota, OfferTypeKey,
    PrismaClient,
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

    pub async fn transaction<TRet, TFut, TFn>(
        &self,
        tx: TFn,
    ) -> Result<TRet, Error>
    where
        TFut: Future<Output = Result<TRet, Error>>,
        TFn: FnOnce(PrismaClient) -> TFut,
    {
        self.db._transaction().run(tx).await
    }

    pub async fn create_offer(
        &self,
        owner: &str,
        details: &Details,
        offer_type_kind: &OfferTypeKind,
    ) -> Result<String, Error> {
        let offer = self
            .db
            .offer()
            .create(owner.to_owned(), offer_type_kind.to_owned().into(), vec![])
            .exec()
            .await?;

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

        Ok(offer.offer_id)
    }

    pub async fn get_offer(
        &self,
        offer_id: &str,
    ) -> Result<Option<offer::Data>, Error> {
        Ok(self
            .db
            .offer()
            .find_unique(offer::offer_id::equals(offer_id.to_owned()))
            .exec()
            .await?)
    }

    pub async fn get_extended_offer(
        &self,
        offer_id: &str,
    ) -> Result<Option<offer::Data>, Error> {
        Ok(self
            .db
            .offer()
            .find_unique(offer::offer_id::equals(offer_id.to_owned()))
            .with(offer::details::fetch())
            .with(
                offer::price::fetch()
                    .with(offer_price::price_type_recurring::fetch()),
            )
            .with(offer::shipping_rate::fetch())
            .with(offer::images::fetch(vec![]))
            .with(offer::files::fetch(vec![]))
            .with(offer::shops::fetch(vec![]))
            .exec()
            .await?)
    }

    pub async fn list_extended_offers(
        &self,
        owner: Option<&String>,
        shop_id: Option<&String>,
        skip: i64,
        take: i64,
        filter: Option<(OffersFilterField, String)>,
        order_by: Option<(OffersOrderByField, Direction)>,
    ) -> Result<(Vec<offer::Data>, u32), Error> {
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
            .with(
                offer::price::fetch()
                    .with(offer_price::price_type_recurring::fetch()),
            )
            .with(offer::shipping_rate::fetch())
            .with(offer::images::fetch(vec![]))
            .with(offer::files::fetch(vec![]))
            .with(offer::shops::fetch(vec![]))
            .order_by(order_by)
            .skip(skip)
            .take(take)
            .exec()
            .await?;

        let count: u32 =
            self.db.offer().count(query).exec().await?.try_into()?;

        Ok((offers, count))
    }

    pub async fn update_offer(
        &self,
        offer_id: &str,
        owner: &str,
        details: Option<&Details>,
        offer_type: Option<&OfferType>,
    ) -> Result<(), Error> {
        if let Some(details) = details {
            self.db
                .offer_details()
                .update_many(
                    vec![
                        offer_details::offer_id::equals(offer_id.to_owned()),
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
                    .offer()
                    .upsert(
                        offer::offer_id::equals(offer_id.to_owned()),
                        offer::create(
                            owner.to_owned(),
                            offer_type_kind.into(),
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
        offer_id: &str,
        owner: &str,
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
        offer_id: &str,
        owner: &str,
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
                    price_type.to_owned().into(),
                    offer::offer_id::equals(offer_id.to_owned()),
                    vec![],
                ),
                vec![
                    offer_price::unit_amount::set(unit_amount),
                    offer_price::currency::set(currency.to_owned()),
                    offer_price::price_type::set(price_type.to_owned().into()),
                ],
            )
            .exec()
            .await?;

        if let Some(price_type_kind) = price_type.price_type_kind {
            match price_type_kind {
                PriceTypeKind::OneTime(_) => {
                    self.db
                        .price_recurring()
                        .delete_many(vec![price_recurring::offer_id::equals(
                            offer_id.to_owned(),
                        )])
                        .exec()
                        .await?;
                }
                PriceTypeKind::Recurring(recurring) => {
                    self.db
                        .price_recurring()
                        .upsert(
                            price_recurring::offer_id::equals(
                                offer_id.to_owned(),
                            ),
                            price_recurring::create(
                                recurring.interval().as_str_name().to_owned(),
                                recurring.interval_count as i32,
                                offer_price::offer_id::equals(
                                    offer_id.to_owned(),
                                ),
                                vec![],
                            ),
                            vec![],
                        )
                        .exec()
                        .await?;
                }
            }
        }

        Ok(())
    }

    pub async fn delete_offer_price(
        &self,
        offer_id: &str,
        owner: &str,
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

    pub async fn upsert_offer_shipping_rate(
        &self,
        offer_id: &str,
        owner: &str,
        unit_amount: u32,
        currency: &str,
        all_countries: bool,
        specific_countries: &Vec<String>,
    ) -> Result<(), Error> {
        self.db
            .offer_shipping_rate()
            .upsert(
                offer_shipping_rate::offer_id::equals(offer_id.to_owned()),
                offer_shipping_rate::create(
                    owner.to_owned(),
                    unit_amount as i32,
                    currency.to_owned(),
                    all_countries,
                    offer::offer_id::equals(offer_id.to_owned()),
                    vec![offer_shipping_rate::specific_countries::set(
                        specific_countries.to_owned(),
                    )],
                ),
                vec![
                    offer_shipping_rate::unit_amount::set(unit_amount as i32),
                    offer_shipping_rate::currency::set(currency.to_owned()),
                    offer_shipping_rate::all_countries::set(all_countries),
                ],
            )
            .exec()
            .await?;

        Ok(())
    }

    pub async fn delete_offer_shipping_rate(
        &self,
        offer_id: &str,
        owner: &str,
    ) -> Result<(), Error> {
        self.db
            .offer_shipping_rate()
            .delete_many(vec![
                offer_shipping_rate::offer_id::equals(offer_id.to_owned()),
                offer_shipping_rate::owner::equals(owner.to_owned()),
            ])
            .exec()
            .await?;

        Ok(())
    }

    pub async fn create_offer_image(
        client: &PrismaClient,
        offer_image_id: &Uuid,
        offer_id: &str,
        owner: &str,
        image_url: &str,
        ordering: i32,
    ) -> Result<offer_image::Data, Error> {
        Ok(client
            .offer_image()
            .create(
                owner.to_owned(),
                image_url.to_owned(),
                ordering,
                offer::offer_id::equals(offer_id.to_owned()),
                vec![offer_image::offer_image_id::set(
                    offer_image_id.to_string(),
                )],
            )
            .exec()
            .await?)
    }

    pub async fn get_offer_image(
        &self,
        offer_image_id: &str,
    ) -> Result<Option<offer_image::Data>, Error> {
        Ok(self
            .db
            .offer_image()
            .find_unique(offer_image::offer_image_id::equals(
                offer_image_id.to_owned(),
            ))
            .exec()
            .await?)
    }

    pub async fn list_offer_images(
        &self,
        offer_id: &str,
    ) -> Result<Vec<offer_image::Data>, Error> {
        Ok(self
            .db
            .offer_image()
            .find_many(vec![offer_image::offer_id::equals(offer_id.to_owned())])
            .exec()
            .await?)
    }

    pub async fn update_offer_image_ordering(
        &self,
        offer_image_id: &str,
        ordering: i32,
    ) -> Result<(), Error> {
        self.db
            .offer_image()
            .update(
                offer_image::offer_image_id::equals(offer_image_id.to_owned()),
                vec![offer_image::ordering::set(ordering)],
            )
            .exec()
            .await?;

        Ok(())
    }

    pub async fn delete_offer_image(
        client: &PrismaClient,
        offer_image_id: &str,
        owner: &str,
    ) -> Result<(), Error> {
        client
            .offer_image()
            .delete_many(vec![
                offer_image::offer_image_id::equals(offer_image_id.to_owned()),
                offer_image::owner::equals(owner.to_owned()),
            ])
            .exec()
            .await?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_offer_file(
        client: &PrismaClient,
        offer_file_id: &Uuid,
        offer_id: &str,
        owner: &str,
        file_name: &str,
        content_type: Option<&String>,
        total_size_bytes: usize,
        uploaded_size_bytes: usize,
        file_path: &str,
        file_url: &str,
        ordering: i32,
    ) -> Result<offer_file::Data, Error> {
        Ok(client
            .offer_file()
            .create(
                owner.to_owned(),
                file_name.to_owned(),
                total_size_bytes as i64,
                uploaded_size_bytes as i64,
                file_path.to_owned(),
                file_url.to_owned(),
                ordering,
                offer::offer_id::equals(offer_id.to_owned()),
                vec![
                    offer_file::offer_file_id::set(offer_file_id.to_string()),
                    offer_file::content_type::set(content_type.cloned()),
                ],
            )
            .exec()
            .await?)
    }

    pub async fn get_offer_file(
        &self,
        offer_file_id: &str,
    ) -> Result<Option<offer_file::Data>, Error> {
        Ok(self
            .db
            .offer_file()
            .find_unique(offer_file::offer_file_id::equals(
                offer_file_id.to_owned(),
            ))
            .exec()
            .await?)
    }

    pub async fn get_accessible_offer_file(
        &self,
        offer_file_id: &str,
        user_id: &str,
    ) -> Result<Option<offer_file::Data>, Error> {
        Ok(self
            .db
            .offer_file()
            .find_first(vec![
                offer_file::owner::equals(user_id.to_owned()),
                offer_file::offer_file_id::equals(offer_file_id.to_owned()),
            ])
            .exec()
            .await?)
    }

    pub async fn list_files_by_offer(
        &self,
        offer_id: &str,
    ) -> Result<Vec<offer_file::Data>, Error> {
        Ok(self
            .db
            .offer_file()
            .find_many(vec![offer_file::offer_id::equals(offer_id.to_owned())])
            .exec()
            .await?)
    }

    pub async fn list_files_by_owner(
        &self,
        owner: &str,
    ) -> Result<Vec<offer_file::Data>, Error> {
        Ok(self
            .db
            .offer_file()
            .find_many(vec![offer_file::owner::equals(owner.to_owned())])
            .order_by(offer_file::OrderByParam::Ordering(
                prisma_client_rust::Direction::Asc,
            ))
            .exec()
            .await?)
    }

    pub async fn update_offer_file_size(
        &self,
        offer_file_id: &str,
        uploaded_size_bytes: usize,
    ) -> Result<offer_file::Data, Error> {
        Ok(self
            .db
            .offer_file()
            .update(
                offer_file::offer_file_id::equals(offer_file_id.to_owned()),
                vec![offer_file::uploaded_size_bytes::set(
                    uploaded_size_bytes as i64,
                )],
            )
            .exec()
            .await?)
    }

    pub async fn update_offer_file_ordering(
        &self,
        offer_file_id: &str,
        ordering: i32,
    ) -> Result<(), Error> {
        self.db
            .offer_file()
            .update(
                offer_file::offer_file_id::equals(offer_file_id.to_owned()),
                vec![offer_file::ordering::set(ordering)],
            )
            .exec()
            .await?;

        Ok(())
    }

    pub async fn delete_offer_file(
        client: &PrismaClient,
        offer_file_id: &str,
    ) -> Result<(), Error> {
        client
            .offer_file()
            .delete(offer_file::offer_file_id::equals(offer_file_id.to_owned()))
            .exec()
            .await?;

        Ok(())
    }

    pub async fn create_shop(
        &self,
        owner: &str,
        website_id: &str,
    ) -> Result<shop::Data, Error> {
        Ok(self
            .db
            .shop()
            .create(owner.to_owned(), website_id.to_owned(), vec![])
            .with(shop::offers::fetch(vec![]))
            .exec()
            .await?)
    }

    pub async fn get_shop(
        &self,
        shop_id: &str,
    ) -> Result<Option<shop::Data>, Error> {
        Ok(self
            .db
            .shop()
            .find_unique(shop::shop_id::equals(shop_id.to_owned()))
            .with(shop::offers::fetch(vec![]))
            .exec()
            .await?)
    }

    pub async fn delete_shop(
        &self,
        shop_id: &str,
        owner: &str,
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
        offer_id: &str,
        shop_id: &str,
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
        offer_id: &str,
        shop_id: &str,
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

    pub async fn get_order(
        &self,
        order_id: &str,
    ) -> Result<Option<order::Data>, Error> {
        Ok(self
            .db
            .order()
            .find_unique(order::order_id::equals(order_id.to_owned()))
            .with(order::order_type_one_off::fetch())
            .with(order::order_type_subscription::fetch())
            .exec()
            .await?)
    }

    pub async fn list_orders(
        &self,
        user_id: &str,
        offer_id: Option<&String>,
    ) -> Result<Vec<order::Data>, Error> {
        let mut query = Vec::with_capacity(2);
        query.push(order::buyer_user_id::equals(user_id.to_owned()));

        if let Some(offer_id) = offer_id {
            query.push(order::offer_id::equals(offer_id.to_owned()));
        }

        Ok(self
            .db
            .order()
            .find_many(query)
            .with(order::order_type_one_off::fetch())
            .with(order::order_type_subscription::fetch())
            .exec()
            .await?)
    }

    pub async fn create_stripe_account(
        client: &PrismaClient,
        stripe_account_id: &str,
        website_id: &str,
        owner: &str,
    ) -> Result<stripe_account::Data, Error> {
        Ok(client
            .stripe_account()
            .create(
                stripe_account_id.to_owned(),
                website_id.to_owned(),
                owner.to_owned(),
                crate::prisma::StripeAccountStatus::Pending,
                vec![],
            )
            .with(stripe_account::status_pending::fetch())
            .with(stripe_account::status_configured::fetch())
            .exec()
            .await?)
    }

    pub async fn add_link_to_stripe_account(
        client: &PrismaClient,
        stripe_account_id: &str,
        link: &str,
    ) -> Result<stripe_account::Data, Error> {
        client
            .stripe_account_status_pending()
            .upsert(
                stripe_account_status_pending::stripe_account_id::equals(
                    stripe_account_id.to_owned(),
                ),
                stripe_account_status_pending::create(
                    link.to_owned(),
                    stripe_account::stripe_account_id::equals(
                        stripe_account_id.to_owned(),
                    ),
                    vec![],
                ),
                vec![stripe_account_status_pending::link::set(link.to_owned())],
            )
            .exec()
            .await?;

        Ok(client
            .stripe_account()
            .update(
                stripe_account::stripe_account_id::equals(
                    stripe_account_id.to_owned(),
                ),
                vec![stripe_account::status::set(
                    crate::prisma::StripeAccountStatus::Pending,
                )],
            )
            .with(stripe_account::status_pending::fetch())
            .with(stripe_account::status_configured::fetch())
            .exec()
            .await?)
    }

    pub async fn get_stripe_account(
        &self,
        stripe_account_id: &str,
    ) -> Result<Option<stripe_account::Data>, Error> {
        Ok(self
            .db
            .stripe_account()
            .find_unique(stripe_account::stripe_account_id::equals(
                stripe_account_id.to_owned(),
            ))
            .with(stripe_account::status_configured::fetch())
            .with(stripe_account::status_pending::fetch())
            .exec()
            .await?)
    }

    pub async fn get_stripe_account_by_website_id(
        &self,
        website_id: &str,
    ) -> Result<Option<stripe_account::Data>, Error> {
        Ok(self
            .db
            .stripe_account()
            .find_unique(stripe_account::website_id::equals(
                website_id.to_owned(),
            ))
            .with(stripe_account::status_configured::fetch())
            .with(stripe_account::status_pending::fetch())
            .exec()
            .await?)
    }

    pub async fn create_user_quota(
        &self,
        user_id: &str,
        max_allowed_size_bytes: usize,
    ) -> Result<user_quota::Data, Error> {
        Ok(self
            .db
            .user_quota()
            .create(user_id.to_owned(), max_allowed_size_bytes as i64, vec![])
            .exec()
            .await?)
    }

    pub async fn get_user_quota(
        &self,
        user_id: &str,
    ) -> Result<Option<user_quota::Data>, Error> {
        Ok(self
            .db
            .user_quota()
            .find_unique(user_quota::user_id::equals(user_id.to_owned()))
            .exec()
            .await?)
    }

    pub async fn update_user_quota(
        &self,
        user_id: &str,
        uploaded_size_bytes: usize,
    ) -> Result<(), Error> {
        self.db
            .user_quota()
            .update(
                user_quota::user_id::equals(user_id.to_owned()),
                vec![user_quota::uploaded_size_bytes::set(
                    uploaded_size_bytes as i64,
                )],
            )
            .exec()
            .await?;

        Ok(())
    }

    pub async fn upsert_sub_website(
        &self,
        website_id: &str,
        owner: &str,
    ) -> Result<(), Error> {
        self.db
            .sub_webiste()
            .upsert(
                sub_webiste::website_id::equals(website_id.to_owned()),
                sub_webiste::create(
                    website_id.to_owned(),
                    owner.to_owned(),
                    vec![],
                ),
                vec![sub_webiste::owner::set(owner.to_owned())],
            )
            .exec()
            .await?;

        Ok(())
    }

    pub async fn get_sub_website(
        &self,
        website_id: &str,
    ) -> Result<Option<sub_webiste::Data>, Error> {
        Ok(self
            .db
            .sub_webiste()
            .find_unique(sub_webiste::website_id::equals(website_id.to_owned()))
            .exec()
            .await?)
    }
    pub async fn delete_sub_website(
        &self,
        website_id: &str,
    ) -> Result<(), Error> {
        self.db
            .sub_webiste()
            .delete(sub_webiste::website_id::equals(website_id.to_owned()))
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
                if value == OfferTypeKey::Physical.to_string() {
                    query.push(offer::offer_type::equals(
                        OfferTypeKey::Physical,
                    ));
                } else if value == OfferTypeKey::Digital.to_string() {
                    query
                        .push(offer::offer_type::equals(OfferTypeKey::Digital));
                } else {
                    return Err(Error::from(format!(
                        "unknown OfferTypeKey {}",
                        value
                    )));
                };
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
