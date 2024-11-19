use std::time::Duration;

use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use aws_sdk_s3::Client;
use tonic::Status;

#[derive(Debug, Clone)]
pub struct FileService {
    client: Client,
    bucket_name: String,
    max_allowed_image_size_bytes: usize,
}

impl FileService {
    pub async fn init(
        bucket_name: String,
        bucket_endpoint: String,
        access_key_id: String,
        secret_access_key: String,
        max_allowed_image_size_bytes: usize,
    ) -> Self {
        let credentials = Credentials::new(
            access_key_id,
            secret_access_key,
            None,
            None,
            "Static",
        );

        let config = aws_config::defaults(BehaviorVersion::v2024_03_28())
            .credentials_provider(credentials)
            .region(Region::new("auto"))
            .endpoint_url(bucket_endpoint)
            .load()
            .await;

        let client = Client::new(&config);

        Self {
            bucket_name,
            client,
            max_allowed_image_size_bytes,
        }
    }

    pub fn validate_image(&self, image_data: &[u8]) -> Result<(), Status> {
        if image_data.len() > self.max_allowed_image_size_bytes {
            return Err(Status::resource_exhausted(format!(
                "image.size: max_allowed_image_size_bytes={}",
                self.max_allowed_image_size_bytes
            )));
        }

        if !(infer::image::is_jpeg(image_data)
            || infer::image::is_jpeg2000(image_data)
            || infer::image::is_png(image_data)
            || infer::image::is_webp(image_data))
        {
            return Err(Status::invalid_argument(
                "image.type: allowed_types=jpg,png,webp",
            ));
        }

        Ok(())
    }

    pub async fn put_image(
        &self,
        image_path: &str,
        image_data: &[u8],
    ) -> Result<(), Status> {
        let img = image::load_from_memory(image_data).map_err(|err| {
            tracing::error!("[FileService.put_image]: {err}");
            Status::internal("")
        })?;
        let encoder = webp::Encoder::from_image(&img).map_err(|err| {
            tracing::error!("[FileService.put_image]: {err}");
            Status::internal("")
        })?;
        let img_webp = encoder.encode_lossless().to_owned();

        self.put_file(image_path, &img_webp, Some("image/webp"))
            .await?;

        Ok(())
    }

    pub async fn put_file(
        &self,
        file_path: &str,
        file_data: &[u8],
        content_type: Option<impl Into<String>>,
    ) -> Result<(), Status> {
        let mut req = self
            .client
            .put_object()
            .bucket(&self.bucket_name)
            .key(file_path)
            .body(ByteStream::from(file_data.to_vec()));

        if let Some(content_type) = content_type {
            req = req.content_type(content_type);
        }

        req.send().await.map_err(|err| {
            tracing::log::error!("[FileService.put_file]: {err}");
            Status::internal("")
        })?;

        Ok(())
    }

    /// Returns `upload_id`
    pub async fn create_multipart_upload(
        &self,
        key: &str,
        content_type: Option<&String>,
    ) -> Result<String, Status> {
        let mut req = self
            .client
            .create_multipart_upload()
            .bucket(&self.bucket_name)
            .key(key);

        if let Some(content_type) = content_type {
            req = req.content_type(content_type);
        }

        let response = req.send().await.map_err(|err| {
            tracing::log::error!(
                "[FileService.create_multipart_upload]: {err}"
            );
            Status::internal("")
        })?;

        if let Some(upload_id) = response.upload_id {
            Ok(upload_id)
        } else {
            Err(Status::data_loss("upload_id"))
        }
    }

    /// Returns `e_tag`
    pub async fn upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: i32,
        file_data: &[u8],
    ) -> Result<String, Status> {
        let part = self
            .client
            .upload_part()
            .bucket(&self.bucket_name)
            .key(key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(ByteStream::from(file_data.to_vec()))
            .send()
            .await
            .map_err(|err| {
                tracing::log::error!(
                    "[FileService.upload_part]: {:?} {:?}",
                    err.as_service_error(),
                    err.raw_response(),
                );
                Status::internal("")
            })?;

        Ok(part.e_tag.unwrap_or_default())
    }

    pub async fn complete_multipart_upload(
        &self,
        key: &str,
        upload_id: &str,
        parts: Vec<CompletedPart>,
    ) -> Result<(), Status> {
        let completed_multipart_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(parts))
            .build();

        self.client
            .complete_multipart_upload()
            .bucket(&self.bucket_name)
            .key(key)
            .upload_id(upload_id)
            .multipart_upload(completed_multipart_upload)
            .send()
            .await
            .map_err(|err| {
                tracing::log::error!(
                    "[FileService.complete_multipart_upload]: {:?} {:?}",
                    err.as_service_error(),
                    err.raw_response()
                );
                Status::internal("")
            })?;

        Ok(())
    }

    pub async fn abort_multipart_upload(
        &self,
        file_path: &str,
        upload_id: &str,
    ) -> Result<(), Status> {
        self.client
            .abort_multipart_upload()
            .bucket(&self.bucket_name)
            .key(file_path)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(|err| {
                tracing::log::error!(
                    "[FileService.abort_multipart_upload]: {err}"
                );
                Status::internal("")
            })?;

        Ok(())
    }

    pub async fn get_presigned_url(
        &self,
        file_path: &str,
        file_name: &str,
    ) -> Result<String, Status> {
        let presigned_config = PresigningConfig::expires_in(
            Duration::from_secs(1800),
        )
        .map_err(|err| {
            tracing::log::error!("[FileService.get_presigned_url]: {err}");
            Status::internal("")
        })?;

        let uri = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(file_path)
            .response_content_disposition(format!(
                r#"attachment; filename="{file_name}""#
            ))
            .presigned(presigned_config)
            .await
            .map_err(|err| {
                tracing::log::error!("[FileService.get_presigned_url]: {err}");
                Status::internal("")
            })?
            .uri()
            .to_owned();

        Ok(uri)
    }

    pub async fn remove_file(&self, file_path: &str) -> Result<(), Status> {
        self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(file_path)
            .send()
            .await
            .map_err(|err| {
                tracing::log::error!("[FileService.remove_file]: {err}");
                Status::internal("")
            })?;

        Ok(())
    }
}
