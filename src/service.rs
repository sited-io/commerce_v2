use std::collections::HashMap;
use std::fmt::Display;

use aws_sdk_s3::types::CompletedPart;
use tonic::{async_trait, Request, Response, Status};
use uuid::Uuid;

use service_apis::sited_io::commerce::v2::price_type::PriceTypeKind;
use service_apis::sited_io::commerce::v2::*;
use service_apis::sited_io::types::currency::v1::CurrencyCode;

use crate::common::auth::Auth;
use crate::common::query;
use crate::prisma::{
    offer, sub_webiste, OfferTypeKey, OrderTypeKey, PaymentMethodKey,
    StripeAccountStatus,
};
use crate::{prisma, CommerceRepository, FileService, StripeService};

const STRIPE_METHADATA_OFFER_ID_KEY: &str = "offer_id";

pub struct CommerceService {
    auth: Auth,
    repository: CommerceRepository,
    file_service: FileService,
    stripe_service: StripeService,
    base_url: String,
    default_user_quota_max_allowed_size_bytes: usize,
    default_platform_fee_percent: u32,
    default_minimum_platform_fee_cent: u32,
}

impl CommerceService {
    fn metadata_key_user_id() -> String {
        String::from("user_id")
    }

    #[allow(clippy::too_many_arguments)]
    pub fn init(
        auth: Auth,
        repository: CommerceRepository,
        file_service: FileService,
        stripe_service: StripeService,
        base_url: String,
        default_user_quota_max_allowed_size_bytes: usize,
        default_platform_fee_percent: u32,
        default_minimum_platform_fee_cent: u32,
    ) -> commerce_service_server::CommerceServiceServer<Self> {
        commerce_service_server::CommerceServiceServer::new(Self {
            auth,
            repository,
            file_service,
            stripe_service,
            base_url,
            default_user_quota_max_allowed_size_bytes,
            default_platform_fee_percent,
            default_minimum_platform_fee_cent,
        })
    }

    async fn check_offer_owner(
        &self,
        offer_id: &str,
        owner: &str,
    ) -> Result<prisma::offer::Data, Status> {
        let offer = self
            .repository
            .get_offer(offer_id)
            .await?
            .ok_or_else(|| Status::not_found(""))?;
        if offer.owner == *owner {
            Ok(offer)
        } else {
            Err(Status::not_found(""))
        }
    }

    async fn check_offer_file_owner(
        &self,
        offer_file_id: &str,
        owner: &str,
    ) -> Result<prisma::offer_file::Data, Status> {
        let offer_file = self
            .repository
            .get_offer_file(offer_file_id)
            .await?
            .ok_or_else(|| Status::not_found(""))?;

        if offer_file.owner == *owner {
            Ok(offer_file)
        } else {
            Err(Status::not_found(""))
        }
    }

    async fn check_shop_owner(
        &self,
        shop_id: &str,
        owner: &str,
    ) -> Result<prisma::shop::Data, Status> {
        let shop = self
            .repository
            .get_shop(shop_id)
            .await?
            .ok_or_else(|| Status::not_found(""))?;
        if shop.owner == *owner {
            Ok(shop)
        } else {
            Err(Status::not_found(""))
        }
    }

    async fn check_quota(
        &self,
        user_id: &str,
        additional_bytes: usize,
    ) -> Result<(), Status> {
        let user_quota = self.ensure_user_quota(user_id).await?;
        if user_quota.uploaded_size_bytes + additional_bytes as i64
            > user_quota.max_allowed_size_bytes
        {
            Err(Status::out_of_range("quota reached"))
        } else {
            Ok(())
        }
    }

    async fn ensure_user_quota(
        &self,
        user_id: &str,
    ) -> Result<prisma::user_quota::Data, Status> {
        match self.repository.get_user_quota(user_id).await? {
            Some(user_quota) => Ok(user_quota),
            None => Ok(self
                .repository
                .create_user_quota(
                    user_id,
                    self.default_user_quota_max_allowed_size_bytes,
                )
                .await?),
        }
    }

    async fn check_website_owner(
        &self,
        owner: &str,
        website_id: &str,
    ) -> Result<sub_webiste::Data, Status> {
        let website = self
            .repository
            .get_sub_website(website_id)
            .await?
            .ok_or_else(|| Status::not_found(""))?;
        if website.owner == *owner {
            Ok(website)
        } else {
            Err(Status::not_found(""))
        }
    }

    fn validate_price(
        &self,
        unit_amount: u32,
        currency: CurrencyCode,
        price_type: Option<PriceType>,
    ) -> Result<(u32, PriceType), Status> {
        let Some(price_type) = price_type else {
            return Err(Status::invalid_argument("price_type"));
        };
        let Some(price_type_kind) = price_type.price_type_kind else {
            return Err(Status::invalid_argument("price_type.price_type_kind"));
        };
        if currency == CurrencyCode::Unspecified {
            return Err(Status::invalid_argument("price.currency"));
        }
        if let PriceTypeKind::Recurring(recurring) = price_type_kind {
            if recurring.interval < 1 {
                return Err(Status::invalid_argument(
                    "price_type.recurring.interval",
                ));
            }
        }

        Ok((unit_amount, price_type))
    }

    fn build_object_url(&self, file_path: &impl Display) -> String {
        format!("{}/{}", self.base_url, file_path)
    }

    fn build_image_path(
        user_id: &impl Display,
        offer_id: &impl Display,
        offer_image_id: &impl Display,
    ) -> String {
        // /{user_id}/offers/{offer_id}/{offer_image_id}
        format!("/{}/offers/{}/{}", user_id, offer_id, offer_image_id)
    }

    fn build_file_path(
        user_id: &impl Display,
        offer_id: &impl Display,
        offer_file_id: &impl Display,
        file_name: &impl Display,
    ) -> String {
        // /{user_id}/offers/{offer_id}/files/{offer_file_id}/{file_name}
        format!(
            "/{}/offers/{}/files/{}/{}",
            user_id, offer_id, offer_file_id, file_name
        )
    }

    fn calculate_fee_amount(&self, unit_amount: i32) -> i64 {
        i64::from(
            ((unit_amount as u32 * self.default_platform_fee_percent) / 100)
                .max(self.default_minimum_platform_fee_cent),
        )
    }

    fn calculate_fee_percent(&self, unit_amount: i32) -> f64 {
        let fee_amount =
            (unit_amount as u32 * self.default_platform_fee_percent) / 100;
        if fee_amount < self.default_minimum_platform_fee_cent {
            let fee_percent =
                f64::from(self.default_minimum_platform_fee_cent * 100)
                    / f64::from(unit_amount);
            // rouds f64 to second decimal
            (fee_percent * 100.0).round() / 100.0
        } else {
            f64::from(self.default_platform_fee_percent)
        }
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

        let Some(offer_type_kind) = offer_type.and_then(|o| o.offer_type_kind)
        else {
            return Err(Status::invalid_argument(
                "please provide 'offer_type'",
            ));
        };

        let offer_id = self
            .repository
            .create_offer(&user_id, &details, &offer_type_kind)
            .await?;

        let offer = self.repository.get_extended_offer(&offer_id).await?;

        Ok(Response::new(CreateOfferResponse {
            offer: offer.map(Offer::from),
        }))
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

        Ok(Response::new(GetOfferResponse {
            offer: Some(Offer::from(offer)),
        }))
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
            offers: offers.into_iter().map(Offer::from).collect(),
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

        Ok(Response::new(UpdateOfferResponse {
            offer: offer.map(Offer::from),
        }))
    }

    async fn delete_offer(
        &self,
        request: Request<DeleteOfferRequest>,
    ) -> Result<Response<DeleteOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let DeleteOfferRequest { offer_id } = request.into_inner();

        self.repository.delete_offer(&offer_id, &user_id).await?;

        Ok(Response::new(DeleteOfferResponse {}))
    }

    async fn put_price_to_offer(
        &self,
        request: Request<PutPriceToOfferRequest>,
    ) -> Result<Response<PutPriceToOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let PutPriceToOfferRequest {
            offer_id,
            unit_amount,
            price_type,
            currency,
        } = request.into_inner();
        let currency = CurrencyCode::try_from(currency).unwrap();

        let (unit_amount, price_type) =
            self.validate_price(unit_amount, currency, price_type)?;

        self.check_offer_owner(&offer_id, &user_id).await?;

        self.repository
            .upsert_price(
                &offer_id,
                &user_id,
                unit_amount,
                currency.as_str_name(),
                &price_type,
            )
            .await?;

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

        Ok(Response::new(RemovePriceFromOfferResponse {}))
    }

    async fn put_shipping_rate_to_offer(
        &self,
        request: Request<PutShippingRateToOfferRequest>,
    ) -> Result<Response<PutShippingRateToOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let request = request.into_inner();

        let specific_country_codes: Vec<String> = request
            .specific_countries()
            .map(|s| s.as_str_name().to_owned())
            .collect();

        let currency = request.currency();
        let PutShippingRateToOfferRequest {
            offer_id,
            unit_amount,
            all_countries,
            currency: _,
            specific_countries: _,
        } = request;

        self.check_offer_owner(&offer_id, &user_id).await?;

        self.repository
            .upsert_offer_shipping_rate(
                &user_id,
                &offer_id,
                unit_amount,
                currency.as_str_name(),
                all_countries,
                &specific_country_codes,
            )
            .await?;

        Ok(Response::new(PutShippingRateToOfferResponse {}))
    }

    async fn remove_shipping_rate_from_offer(
        &self,
        request: Request<RemoveShippingRateFromOfferRequest>,
    ) -> Result<Response<RemoveShippingRateFromOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let RemoveShippingRateFromOfferRequest { offer_id } =
            request.into_inner();

        self.repository
            .delete_offer_shipping_rate(&offer_id, &user_id)
            .await?;

        Ok(Response::new(RemoveShippingRateFromOfferResponse {}))
    }

    async fn add_image_to_offer(
        &self,
        request: Request<AddImageToOfferRequest>,
    ) -> Result<Response<AddImageToOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let AddImageToOfferRequest {
            offer_id,
            data,
            ordering,
        } = request.into_inner();

        let offer = self.check_offer_owner(&offer_id, &user_id).await?;

        self.file_service.validate_image(&data)?;

        let offer_image_id = Uuid::new_v4();

        let image_path =
            Self::build_image_path(&user_id, &offer.offer_id, &offer_image_id);
        let image_url = self.build_object_url(&image_path);

        self.repository
            .transaction(|client| async move {
                CommerceRepository::create_offer_image(
                    &client,
                    &offer_image_id,
                    &offer_id,
                    &user_id,
                    &image_url,
                    ordering,
                )
                .await?;

                self.file_service.put_image(&image_path, &data).await?;

                Ok(())
            })
            .await?;

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

        let offer = self.check_offer_owner(&offer_id, &user_id).await?;

        let image_path =
            Self::build_image_path(&user_id, &offer.offer_id, &offer_image_id);

        self.repository
            .transaction(|client| async move {
                self.file_service.remove_file(&image_path).await?;

                CommerceRepository::delete_offer_image(
                    &client,
                    &offer_image_id,
                    &user_id,
                )
                .await?;

                Ok(())
            })
            .await?;

        Ok(Response::new(RemoveImageFromOfferResponse {}))
    }

    async fn add_file_to_offer(
        &self,
        request: Request<AddFileToOfferRequest>,
    ) -> Result<Response<AddFileToOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let AddFileToOfferRequest {
            file_name,
            offer_id,
            content_type,
            content,
            ordering,
        } = request.into_inner();

        let offer = self.check_offer_owner(&offer_id, &user_id).await?;
        self.check_quota(&user_id, content.len()).await?;

        let all_files = self.repository.list_offer_files(&user_id).await?;

        let offer_file_id = Uuid::new_v4();

        let file_path = Self::build_file_path(
            &user_id,
            &offer.offer_id,
            &offer_file_id.to_string(),
            &file_name,
        );
        let file_url = self.build_object_url(&file_path);

        let ordering = ordering
            .or_else(|| all_files.last().map(|f| f.ordering + 1))
            .unwrap_or_default();

        self.repository
            .transaction(|client| async move {
                CommerceRepository::create_offer_file(
                    &client,
                    &offer_file_id,
                    &offer.offer_id,
                    &user_id,
                    &file_name,
                    content_type.as_ref(),
                    content.len(),
                    content.len(),
                    &file_path,
                    &file_url,
                    ordering,
                )
                .await?;

                self.file_service
                    .put_file(&file_path, &content, content_type)
                    .await?;

                Ok(())
            })
            .await?;

        Ok(Response::new(AddFileToOfferResponse {}))
    }

    async fn initiate_multipart_upload(
        &self,
        request: Request<InitiateMultipartUploadRequest>,
    ) -> Result<Response<InitiateMultipartUploadResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let InitiateMultipartUploadRequest {
            file_name,
            offer_id,
            total_size_bytes,
            content_type,
            ordering,
        } = request.into_inner();

        let offer = self.check_offer_owner(&offer_id, &user_id).await?;

        self.check_quota(&user_id, total_size_bytes as usize)
            .await?;

        let all_files = self.repository.list_offer_files(&user_id).await?;

        let offer_file_id = Uuid::new_v4();

        let file_path = Self::build_file_path(
            &user_id,
            &offer.offer_id,
            &offer_file_id,
            &file_name,
        );
        let file_url = self.build_object_url(&file_path);

        let ordering = ordering
            .or_else(|| all_files.last().map(|f| f.ordering + 1))
            .unwrap_or_default();

        let (offer_file, upload_id) = self
            .repository
            .transaction(|client| async move {
                let offer_file = CommerceRepository::create_offer_file(
                    &client,
                    &offer_file_id,
                    &offer.offer_id,
                    &user_id,
                    &file_name,
                    content_type.as_ref(),
                    total_size_bytes as usize,
                    0,
                    &file_path,
                    &file_url,
                    ordering,
                )
                .await?;

                let upload_id = self
                    .file_service
                    .initiate_multipart_upload(
                        &file_path,
                        content_type.as_ref(),
                    )
                    .await?;

                Ok((offer_file, upload_id))
            })
            .await?;

        Ok(Response::new(InitiateMultipartUploadResponse {
            offer_file_id: offer_file.offer_file_id,
            key: offer_file.file_url,
            upload_id,
        }))
    }

    async fn put_multipart_chunk(
        &self,
        request: Request<PutMultipartChunkRequest>,
    ) -> Result<Response<PutMultipartChunkResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let PutMultipartChunkRequest {
            offer_file_id,
            upload_id,
            part_number,
            chunk,
        } = request.into_inner();

        let offer_file = self
            .check_offer_file_owner(&offer_file_id, &user_id)
            .await?;

        let uploaded_size_bytes =
            chunk.len() + offer_file.uploaded_size_bytes as usize;

        let etag = self
            .repository
            .transaction(|client| async move {
                CommerceRepository::update_offer_file_size(
                    &client,
                    &offer_file.offer_file_id,
                    uploaded_size_bytes,
                )
                .await?;

                let etag = self
                    .file_service
                    .put_multipart_chunk(
                        &offer_file.file_url,
                        &upload_id,
                        part_number,
                        &chunk,
                    )
                    .await?;

                Ok(etag)
            })
            .await?;

        Ok(Response::new(PutMultipartChunkResponse {
            part: Some(MultipartPart { part_number, etag }),
        }))
    }

    async fn complete_multipart_upload(
        &self,
        request: Request<CompleteMultipartUploadRequest>,
    ) -> Result<Response<CompleteMultipartUploadResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let CompleteMultipartUploadRequest {
            offer_file_id,
            upload_id,
            parts,
        } = request.into_inner();

        let offer_file = self
            .check_offer_file_owner(&offer_file_id, &user_id)
            .await?;

        if offer_file.total_size_bytes != offer_file.uploaded_size_bytes {
            return Err(Status::failed_precondition(
                "uploaded size not equal to total size",
            ));
        }

        let parts = parts
            .into_iter()
            .map(|p| {
                CompletedPart::builder()
                    .e_tag(p.etag)
                    .part_number(p.part_number)
                    .build()
            })
            .collect();

        self.file_service
            .complete_multipart_upload(&offer_file.file_url, &upload_id, parts)
            .await?;

        Ok(Response::new(CompleteMultipartUploadResponse {}))
    }

    async fn download_file(
        &self,
        request: Request<DownloadFileRequest>,
    ) -> Result<Response<DownloadFileResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let DownloadFileRequest { offer_file_id } = request.into_inner();

        let offer_file = self
            .repository
            .get_accessible_offer_file(&offer_file_id, &user_id)
            .await?
            .ok_or_else(|| Status::not_found(""))?;

        let download_url = self
            .file_service
            .get_presigned_url(&offer_file.file_path, &offer_file.file_name)
            .await?;

        Ok(Response::new(DownloadFileResponse { download_url }))
    }

    async fn update_file_offer_ordering(
        &self,
        request: Request<UpdateFileOfferOrderingRequest>,
    ) -> Result<Response<UpdateFileOfferOrderingResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let UpdateFileOfferOrderingRequest {
            offer_file_id,
            ordering,
        } = request.into_inner();

        self.check_offer_file_owner(&offer_file_id, &user_id)
            .await?;

        self.repository
            .update_offer_file_ordering(&offer_file_id, ordering)
            .await?;

        Ok(Response::new(UpdateFileOfferOrderingResponse {}))
    }

    async fn remove_file_from_offer(
        &self,
        request: Request<RemoveFileFromOfferRequest>,
    ) -> Result<Response<RemoveFileFromOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let RemoveFileFromOfferRequest { offer_file_id } = request.into_inner();

        let offer_file = self
            .check_offer_file_owner(&offer_file_id, &user_id)
            .await?;

        self.repository
            .transaction(|client| async move {
                self.file_service.remove_file(&offer_file.file_path).await?;

                CommerceRepository::delete_offer_file(&client, &offer_file_id)
                    .await?;

                Ok(())
            })
            .await?;

        Ok(Response::new(RemoveFileFromOfferResponse {}))
    }

    async fn create_shop(
        &self,
        request: Request<CreateShopRequest>,
    ) -> Result<Response<CreateShopResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let CreateShopRequest { website_id } = request.into_inner();

        let shop = self.repository.create_shop(&user_id, &website_id).await?;

        Ok(Response::new(CreateShopResponse {
            shop: Some(Shop::from(shop)),
        }))
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

        Ok(Response::new(GetShopResponse {
            shop: Some(Shop::from(shop)),
        }))
    }

    async fn delete_shop(
        &self,
        request: Request<DeleteShopRequest>,
    ) -> Result<Response<DeleteShopResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let DeleteShopRequest { shop_id } = request.into_inner();

        self.repository.delete_shop(&shop_id, &user_id).await?;

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

    async fn get_order(
        &self,
        request: Request<GetOrderRequest>,
    ) -> Result<Response<GetOrderResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let GetOrderRequest { order_id } = request.into_inner();

        let order = self
            .repository
            .get_order(&order_id)
            .await?
            .ok_or_else(|| Status::not_found(""))?;

        if order.buyer_user_id != user_id {
            return Err(Status::not_found(""));
        }

        Ok(Response::new(GetOrderResponse {
            order: Some(Order::from(order)),
        }))
    }

    async fn list_orders(
        &self,
        request: Request<ListOrdersRequest>,
    ) -> Result<Response<ListOrdersResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let ListOrdersRequest { offer_id } = request.into_inner();

        let orders = self
            .repository
            .list_orders(&user_id, offer_id.as_ref())
            .await?;

        Ok(Response::new(ListOrdersResponse {
            orders: orders.into_iter().map(Order::from).collect(),
        }))
    }

    async fn create_stripe_account(
        &self,
        request: Request<CreateStripeAccountRequest>,
    ) -> Result<Response<CreateStripeAccountResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let CreateStripeAccountRequest {
            website_id,
            refresh_url,
            return_url,
        } = request.into_inner();

        self.check_website_owner(&user_id, &website_id).await?;

        match self
            .repository
            .get_stripe_account_by_website_id(&website_id)
            .await?
        {
            None => {
                let stripe_account = self
                    .repository
                    .transaction(|client| async move {
                        let account =
                            self.stripe_service.create_account().await?;

                        CommerceRepository::create_stripe_account(
                            &client,
                            &account.id,
                            &website_id,
                            &user_id,
                        )
                        .await
                    })
                    .await?;

                let stripe_account = self
                    .repository
                    .transaction(|client| async move {
                        let link = self
                            .stripe_service
                            .create_account_link(
                                &stripe_account.stripe_account_id,
                                &refresh_url,
                                &return_url,
                            )
                            .await?;

                        CommerceRepository::add_link_to_stripe_account(
                            &client,
                            &stripe_account.stripe_account_id,
                            &link,
                        )
                        .await
                    })
                    .await?;

                return Ok(Response::new(CreateStripeAccountResponse {
                    stripe_account: Some(stripe_account.into()),
                }));
            }
            Some(stripe_account) => {
                if stripe_account.owner != user_id {
                    return Err(Status::not_found(""));
                }
                if stripe_account.status == StripeAccountStatus::Configured
                    || matches!(
                        &stripe_account.status_configured,
                        Some(Some(_))
                    )
                {
                    return Ok(Response::new(CreateStripeAccountResponse {
                        stripe_account: Some(stripe_account.into()),
                    }));
                }

                let stripe_account = self
                    .repository
                    .transaction(|client| async move {
                        let link = self
                            .stripe_service
                            .create_account_link(
                                &stripe_account.stripe_account_id,
                                &refresh_url,
                                &return_url,
                            )
                            .await?;

                        CommerceRepository::add_link_to_stripe_account(
                            &client,
                            &stripe_account.stripe_account_id,
                            &link,
                        )
                        .await
                    })
                    .await?;

                Ok(Response::new(CreateStripeAccountResponse {
                    stripe_account: Some(stripe_account.into()),
                }))
            }
        }
    }

    async fn get_stripe_account(
        &self,
        request: Request<GetStripeAccountRequest>,
    ) -> Result<Response<GetStripeAccountResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let GetStripeAccountRequest { website_id } = request.into_inner();

        let stripe_account = self
            .repository
            .get_stripe_account_by_website_id(&website_id)
            .await?;

        if matches!(&stripe_account, Some(a) if a.owner != user_id) {
            return Err(Status::not_found(""));
        }

        Ok(Response::new(GetStripeAccountResponse {
            stripe_account: stripe_account.map(StripeAccount::from),
        }))
    }

    async fn buy_offer(
        &self,
        request: Request<BuyOfferRequest>,
    ) -> Result<Response<BuyOfferResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await.ok();

        let BuyOfferRequest {
            offer_id,
            shop_id,
            payment_method,
        } = request.into_inner();

        let Some(payment_method) = payment_method else {
            return Err(Status::invalid_argument("No payment_method provided"));
        };

        let shop =
            self.repository.get_shop(&shop_id).await?.ok_or_else(|| {
                Status::not_found(format!(
                    "Could not find shop by shop_id '{}'",
                    shop_id
                ))
            })?;

        let offer = self
            .repository
            .get_extended_offer(&offer_id)
            .await?
            .ok_or_else(|| {
                Status::not_found(format!(
                    "Could not find offer by offer_id '{}'",
                    offer_id
                ))
            })?;

        let offer::Data {
            offer_id, price, ..
        } = offer.clone();

        let price = price.flatten().ok_or_else(|| {
            Status::failed_precondition(format!(
                "Could not find price for offer by offer_id '{}'",
                offer_id
            ))
        })?;

        let stripe_account = self
            .repository
            .get_stripe_account(&shop.website_id)
            .await?
            .ok_or_else(|| {
                Status::not_found(format!(
                    "Could not find stripe account by website_id '{}'",
                    shop.website_id
                ))
            })?;

        // Add offer_id to metadata of stripe checkout session
        // this is used in stripe webhook handler to assign offers to payments
        let mut metadata = HashMap::from([(
            STRIPE_METHADATA_OFFER_ID_KEY.to_owned(),
            offer_id,
        )]);

        if let OfferTypeKey::Digital = offer.offer_type {
            // If offer type is digital, we need to provide the user_id to the payment
            // in order to assing ownership of the subscription to the buyer.
            // In other cases customers should be able to buy without authentication.
            if let Some(user_id) = user_id {
                metadata.insert(Self::metadata_key_user_id(), user_id);
            } else {
                return Err(Status::unauthenticated(""));
            }
        }

        match payment_method {
            buy_offer_request::PaymentMethod::Stripe(
                buy_offer_request::Stripe {
                    success_url,
                    cancel_url,
                },
            ) => {
                let link = self
                    .stripe_service
                    .create_checkout_session(
                        stripe_account.stripe_account_id,
                        success_url,
                        cancel_url,
                        metadata,
                        self.calculate_fee_amount(price.unit_amount),
                        self.calculate_fee_percent(price.unit_amount),
                        offer,
                        *price,
                    )
                    .await?;

                Ok(Response::new(BuyOfferResponse {
                    payment_method: Some(
                        buy_offer_response::PaymentMethod::Stripe(
                            buy_offer_response::Stripe { link },
                        ),
                    ),
                }))
            }
        }
    }

    async fn cancel_subscription(
        &self,
        request: Request<CancelSubscriptionRequest>,
    ) -> Result<Response<CancelSubscriptionResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let CancelSubscriptionRequest { order_id } = request.into_inner();

        let order =
            self.repository.get_order(&order_id).await?.ok_or_else(|| {
                Status::not_found(format!(
                    "Could not find order by order_id '{}'",
                    order_id
                ))
            })?;

        if order.buyer_user_id != user_id {
            return Err(Status::not_found(format!(
                "Could not find order by order_id '{}'",
                order_id
            )));
        }

        if order.order_type != OrderTypeKey::Subscription {
            return Err(Status::failed_precondition(format!(
                "Order '{}' is not a subscription",
                order_id
            )));
        }

        let order_type_subscription = order.order_type_subscription.clone().flatten().ok_or_else(
            || {
                tracing::error!("Order '{}' was of order_type == Subscription, but had no order_type_subscription",
            order_id);
            Status::internal("")
            }
        )?;
        if order_type_subscription.cancelled_at.is_some() {
            return Err(Status::failed_precondition(format!(
                "Order '{}' is already cancelled",
                order_id
            )));
        }

        match order.payment_method {
            PaymentMethodKey::Stripe => {
                let stripe_account_id = order.payment_method_stripe.clone().flatten().and_then(|p| p.stripe_subscription_id).ok_or_else(
                    || Status::internal(format!(
                        "Order '{}' had payment_method == Stripe, but had no payment_method_stripe",order_id
                    ))
                )?;

                self.stripe_service
                    .update_subscription_period_end(
                        stripe_account_id,
                        order,
                        true,
                    )
                    .await?;

                Ok(Response::new(CancelSubscriptionResponse {}))
            }
        }
    }

    async fn resume_subscription(
        &self,
        request: Request<ResumeSubscriptionRequest>,
    ) -> Result<Response<ResumeSubscriptionResponse>, Status> {
        let user_id = self.auth.get_user_id(&request).await?;

        let ResumeSubscriptionRequest { order_id } = request.into_inner();

        let order =
            self.repository.get_order(&order_id).await?.ok_or_else(|| {
                Status::not_found(format!(
                    "Could not find order by order_id '{}'",
                    order_id
                ))
            })?;

        if order.buyer_user_id != user_id {
            return Err(Status::not_found(format!(
                "Could not find order by order_id '{}'",
                order_id
            )));
        }

        if order.order_type != OrderTypeKey::Subscription {
            return Err(Status::failed_precondition(format!(
                "Order '{}' is not a subscription",
                order_id
            )));
        }

        let order_type_subscription = order.order_type_subscription.clone().flatten().ok_or_else(
            || {
                tracing::error!("Order '{}' was of order_type == Subscription, but had no order_type_subscription",
            order_id);
            Status::internal("")
            }
        )?;
        if order_type_subscription.cancelled_at.is_some() {
            return Err(Status::failed_precondition(format!(
                "Order '{}' is already cancelled",
                order_id
            )));
        }

        match order.payment_method {
            PaymentMethodKey::Stripe => {
                let stripe_account_id = order.payment_method_stripe.clone().flatten().and_then(|p| p.stripe_subscription_id).ok_or_else(
                    || Status::internal(format!(
                        "Order '{}' had payment_method == Stripe, but had no payment_method_stripe",order_id
                    ))
                )?;

                self.stripe_service
                    .update_subscription_period_end(
                        stripe_account_id,
                        order,
                        false,
                    )
                    .await?;

                Ok(Response::new(ResumeSubscriptionResponse {}))
            }
        }
    }
}
