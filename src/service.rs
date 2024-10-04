use tonic::{async_trait, Request, Response, Status};

use service_apis::sited_io::commerce::v2::price_type::PriceTypeKind;
use service_apis::sited_io::commerce::v2::{
    commerce_service_server, AddImageToOfferRequest, AddImageToOfferResponse,
    AddOfferToShopRequest, AddOfferToShopResponse,
    AddShippingRateToOfferRequest, AddShippingRateToOfferResponse,
    CreateOfferRequest, CreateOfferResponse, CreateShopRequest,
    CreateShopResponse, DeleteOfferRequest, DeleteOfferResponse,
    DeleteShopRequest, DeleteShopResponse, GetOfferRequest, GetOfferResponse,
    GetShopRequest, GetShopResponse, ListOffersRequest, ListOffersResponse,
    PriceType, PutPriceToOfferRequest, PutPriceToOfferResponse,
    RemoveImageFromOfferRequest, RemoveImageFromOfferResponse,
    RemoveOfferFromShopRequest, RemoveOfferFromShopResponse,
    RemovePriceFromOfferRequest, RemovePriceFromOfferResponse,
    RemoveShippingRateFromOfferRequest, RemoveShippingRateFromOfferResponse,
    UpdateOfferRequest, UpdateOfferResponse,
};
use service_apis::sited_io::price::v1::{CurrencyCode, Price};

use crate::common::auth::Auth;
use crate::common::query;
use crate::{CommerceRepository, Publisher};

pub struct CommerceService {
    auth: Auth,
    repository: CommerceRepository,
    publisher: Publisher,
}

impl CommerceService {
    pub fn init(
        auth: Auth,
        repository: CommerceRepository,
        publisher: Publisher,
    ) -> commerce_service_server::CommerceServiceServer<Self> {
        commerce_service_server::CommerceServiceServer::new(Self {
            auth,
            repository,
            publisher,
        })
    }
}

#[async_trait]
impl commerce_service_server::CommerceService for CommerceService {
    async fn create_offer(
        &self,
        request: Request<CreateOfferRequest>,
    ) -> Result<Response<CreateOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let CreateOfferRequest {
            details,
            offer_type,
        } = request.into_inner();

        let Some(details) = details else {
            return Err(Status::invalid_argument("please provide 'details'"));
        };

        let Some(offer_type) = offer_type else {
            return Err(Status::invalid_argument(
                "please provide 'offer_type'",
            ));
        };

        let offer_id = self
            .repository
            .create_offer(&user_id, &details, &offer_type)
            .await?;

        let offer = self.repository.get_extended_offer(&offer_id).await?;

        self.publisher.publish_offer_upsert(offer.as_ref()).await;

        Ok(Response::new(CreateOfferResponse { offer }))
    }

    async fn get_offer(
        &self,
        request: Request<GetOfferRequest>,
    ) -> Result<Response<GetOfferResponse>, Status> {
        let GetOfferRequest { offer_id } = request.into_inner();

        let offer = self
            .repository
            .get_extended_offer(&offer_id)
            .await?
            .ok_or_else(|| Status::not_found(""))?;

        Ok(Response::new(GetOfferResponse { offer: Some(offer) }))
    }

    async fn list_offers(
        &self,
        request: Request<ListOffersRequest>,
    ) -> Result<Response<ListOffersResponse>, Status> {
        let ListOffersRequest {
            owner,
            shop_id,
            pagination,
            filter,
            order_by,
        } = request.into_inner();

        let (skip, take, mut pagination) = query::paginate(pagination)?;

        let filter = filter.map(|f| (f.field(), f.query));

        let order_by = order_by.map(|o| (o.field(), o.direction()));

        let (offers, count) = self
            .repository
            .list_extended_offers(
                owner.as_ref(),
                shop_id.as_ref(),
                skip,
                take,
                filter,
                order_by,
            )
            .await?;

        pagination.total_elements = count;

        Ok(Response::new(ListOffersResponse {
            offers,
            pagination: Some(pagination),
        }))
    }

    async fn update_offer(
        &self,
        request: Request<UpdateOfferRequest>,
    ) -> Result<Response<UpdateOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let UpdateOfferRequest {
            offer_id,
            details,
            offer_type,
        } = request.into_inner();

        self.repository
            .update_offer(
                &offer_id,
                &user_id,
                details.as_ref(),
                offer_type.as_ref(),
            )
            .await?;

        let offer = self.repository.get_extended_offer(&offer_id).await?;

        self.publisher.publish_offer_upsert(offer.as_ref()).await;

        Ok(Response::new(UpdateOfferResponse { offer }))
    }

    async fn delete_offer(
        &self,
        request: Request<DeleteOfferRequest>,
    ) -> Result<Response<DeleteOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let DeleteOfferRequest { offer_id } = request.into_inner();

        self.repository.delete_offer(&offer_id, &user_id).await?;

        let offer = self.repository.get_extended_offer(&offer_id).await?;

        self.publisher.publish_offer_delete(offer.as_ref()).await;

        Ok(Response::new(DeleteOfferResponse {}))
    }

    async fn put_price_to_offer(
        &self,
        request: Request<PutPriceToOfferRequest>,
    ) -> Result<Response<PutPriceToOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let PutPriceToOfferRequest {
            offer_id,
            price,
            price_type,
        } = request.into_inner();

        let (price, price_type) = self.validate_price(price, price_type)?;

        self.check_offer_owner(&offer_id, &user_id).await?;

        self.repository
            .upsert_price(
                &offer_id,
                &user_id,
                price.unit_amount,
                price.currency_code().as_str_name(),
                &price_type,
            )
            .await?;

        self.query_and_publish_offer_upsert(&offer_id).await?;

        Ok(Response::new(PutPriceToOfferResponse {}))
    }

    async fn remove_price_from_offer(
        &self,
        request: Request<RemovePriceFromOfferRequest>,
    ) -> Result<Response<RemovePriceFromOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let RemovePriceFromOfferRequest { offer_id } = request.into_inner();

        self.repository
            .delete_offer_price(&offer_id, &user_id)
            .await?;

        self.query_and_publish_offer_upsert(&offer_id).await?;

        Ok(Response::new(RemovePriceFromOfferResponse {}))
    }

    async fn add_shipping_rate_to_offer(
        &self,
        request: Request<AddShippingRateToOfferRequest>,
    ) -> Result<Response<AddShippingRateToOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let request = request.into_inner();

        let specific_country_codes: Vec<String> = request
            .specific_countries()
            .map(|s| s.as_str_name().to_owned())
            .collect();

        let AddShippingRateToOfferRequest {
            offer_id,
            price,
            all_countries,
            specific_countries: _,
        } = request;

        let Some(price) = price else {
            return Err(Status::invalid_argument("price"));
        };

        self.check_offer_owner(&offer_id, &user_id).await?;

        self.repository
            .create_shipping_rate(
                &user_id,
                &offer_id,
                price.unit_amount,
                price.currency_code().as_str_name(),
                all_countries,
                &specific_country_codes,
            )
            .await?;

        self.query_and_publish_offer_upsert(&offer_id).await?;

        Ok(Response::new(AddShippingRateToOfferResponse {}))
    }

    async fn remove_shipping_rate_from_offer(
        &self,
        request: Request<RemoveShippingRateFromOfferRequest>,
    ) -> Result<Response<RemoveShippingRateFromOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let RemoveShippingRateFromOfferRequest {
            shipping_rate_id,
            offer_id,
        } = request.into_inner();

        self.repository
            .delete_shipping_rate(&shipping_rate_id, &user_id)
            .await?;

        self.query_and_publish_offer_upsert(&offer_id).await?;

        Ok(Response::new(RemoveShippingRateFromOfferResponse {}))
    }

    async fn add_image_to_offer(
        &self,
        request: Request<AddImageToOfferRequest>,
    ) -> Result<Response<AddImageToOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let AddImageToOfferRequest {
            offer_id,
            file_id,
            ordering,
        } = request.into_inner();

        self.check_offer_owner(&offer_id, &user_id).await?;

        self.repository
            .create_offer_image(&offer_id, &file_id, &user_id, ordering)
            .await?;

        self.query_and_publish_offer_upsert(&offer_id).await?;

        Ok(Response::new(AddImageToOfferResponse {}))
    }

    async fn remove_image_from_offer(
        &self,
        request: Request<RemoveImageFromOfferRequest>,
    ) -> Result<Response<RemoveImageFromOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let RemoveImageFromOfferRequest {
            offer_image_id,
            offer_id,
        } = request.into_inner();

        self.repository
            .delete_offer_image(&offer_image_id, &user_id)
            .await?;

        self.query_and_publish_offer_upsert(&offer_id).await?;

        Ok(Response::new(RemoveImageFromOfferResponse {}))
    }

    async fn create_shop(
        &self,
        request: Request<CreateShopRequest>,
    ) -> Result<Response<CreateShopResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let CreateShopRequest { website_id } = request.into_inner();

        let shop = self.repository.create_shop(&user_id, &website_id).await?;

        self.publisher.publish_shop_upsert(Some(&shop)).await;

        Ok(Response::new(CreateShopResponse { shop: Some(shop) }))
    }

    async fn get_shop(
        &self,
        request: Request<GetShopRequest>,
    ) -> Result<Response<GetShopResponse>, Status> {
        let GetShopRequest { shop_id } = request.into_inner();

        let shop = self
            .repository
            .get_shop(&shop_id)
            .await?
            .ok_or_else(|| Status::not_found(""))?;

        Ok(Response::new(GetShopResponse { shop: Some(shop) }))
    }

    async fn delete_shop(
        &self,
        request: Request<DeleteShopRequest>,
    ) -> Result<Response<DeleteShopResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let DeleteShopRequest { shop_id } = request.into_inner();

        self.repository.delete_shop(&shop_id, &user_id).await?;

        let shop = self.repository.get_shop(&shop_id).await?;

        self.publisher.publish_shop_delete(shop.as_ref()).await;

        Ok(Response::new(DeleteShopResponse {}))
    }

    async fn add_offer_to_shop(
        &self,
        request: Request<AddOfferToShopRequest>,
    ) -> Result<Response<AddOfferToShopResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let AddOfferToShopRequest { offer_id, shop_id } = request.into_inner();

        self.check_offer_owner(&offer_id, &user_id).await?;
        self.check_shop_owner(&shop_id, &user_id).await?;

        self.repository
            .add_offer_to_shop(&offer_id, &shop_id)
            .await?;

        Ok(Response::new(AddOfferToShopResponse {}))
    }

    async fn remove_offer_from_shop(
        &self,
        request: Request<RemoveOfferFromShopRequest>,
    ) -> Result<Response<RemoveOfferFromShopResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let RemoveOfferFromShopRequest { offer_id, shop_id } =
            request.into_inner();

        self.check_offer_owner(&offer_id, &user_id).await?;
        self.check_shop_owner(&shop_id, &user_id).await?;

        self.repository
            .remove_offer_from_shop(&offer_id, &shop_id)
            .await?;

        Ok(Response::new(RemoveOfferFromShopResponse {}))
    }
}

impl CommerceService {
    async fn check_offer_owner(
        &self,
        offer_id: &str,
        owner: &String,
    ) -> Result<(), Status> {
        let offer = self
            .repository
            .get_offer(offer_id)
            .await?
            .ok_or_else(|| Status::not_found(""))?;
        if offer.owner == *owner {
            Ok(())
        } else {
            Err(Status::not_found(""))
        }
    }

    async fn check_shop_owner(
        &self,
        shop_id: &String,
        owner: &String,
    ) -> Result<(), Status> {
        let shop = self
            .repository
            .get_shop(shop_id)
            .await?
            .ok_or_else(|| Status::not_found(""))?;
        if shop.owner == *owner {
            Ok(())
        } else {
            Err(Status::not_found(""))
        }
    }

    async fn query_and_publish_offer_upsert(
        &self,
        offer_id: &String,
    ) -> Result<(), Status> {
        let offer = self.repository.get_extended_offer(offer_id).await?;

        self.publisher.publish_offer_upsert(offer.as_ref()).await;

        Ok(())
    }

    fn validate_price(
        &self,
        price: Option<Price>,
        price_type: Option<PriceType>,
    ) -> Result<(Price, PriceType), Status> {
        let Some(price) = price else {
            return Err(Status::invalid_argument("price"));
        };
        let Some(price_type) = price_type else {
            return Err(Status::invalid_argument("price_type"));
        };
        let Some(price_type_kind) = price_type.price_type_kind else {
            return Err(Status::invalid_argument("price_type.price_type_kind"));
        };
        if price.currency_code() == CurrencyCode::Unspecified {
            return Err(Status::invalid_argument("price.currency"));
        }

        if let PriceTypeKind::Recurring(recurring) = price_type_kind {
            if recurring.interval < 1 {
                return Err(Status::invalid_argument(
                    "price_type.recurring.interval",
                ));
            }
        }

        Ok((price, price_type))
    }
}
