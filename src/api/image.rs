use std::path::Path;
use std::sync::Arc;

use crate::{
    config::EncodingConfig,
    utils::{load_bytes_from_disk, mime_type_for_format},
};
use actix_web::{HttpMessage, HttpResponse, web};
use futures_util::TryStreamExt;
use tokio_util::io::ReaderStream;

const SUPPORTED_FORMATS: &[&str] = &["avif", "jpeg", "jpg", "png", "webp"];

#[actix_web::get("/{filename:.*}")]
#[tracing::instrument(skip_all, fields(filename = %filename), level = "debug")]
pub async fn process_image_request(
    filename: web::Path<String>,
    query_params: web::Query<std::collections::HashMap<String, String>>,
    config: web::Data<Arc<EncodingConfig>>,
    http_client: web::Data<awc::Client>,
) -> actix_web::Result<HttpResponse> {
    // Strip the specified path from the filename if configured
    let filename = if let Some(strip_path) = &config.strip_path {
        filename
            .strip_prefix(strip_path)
            .unwrap_or(&filename)
            .to_string()
    } else {
        filename.to_string()
    };

    // Sanitize the path to prevent directory traversal
    let sanitized_path = Path::new(&filename)
        .components()
        .filter(|comp| matches!(comp, std::path::Component::Normal(_)))
        .fold(Path::new("").to_path_buf(), |acc, comp| {
            acc.join(comp.as_os_str())
        });

    // Join the sanitized path with the root path to get the final file path
    let sanitized_disk_path = Path::new(&config.root_path).join(&sanitized_path);

    // Check if the extension is valid and supported
    let file_ext = sanitized_disk_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());
    if let Some(ext) = &file_ext
        && !SUPPORTED_FORMATS.contains(&ext.as_str())
    {
        return Ok(
            HttpResponse::UnsupportedMediaType().body(format!("Unsupported file format: {}", ext))
        );
    }

    tracing::debug!(
        query_params = ?query_params,
        strip_path = ?config.strip_path,
        root_path = ?config.root_path,
        sanitized_path = ?sanitized_path,
        sanitized_disk_path = ?sanitized_disk_path,
        filename = ?filename,
    );

    let format_param = query_params.get("format").map(|s| s.to_lowercase());
    let size_param = query_params
        .get("size")
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|&s| s > 0);

    // Stream local files directly when no transformation is requested.
    if query_params.is_empty() && sanitized_disk_path.exists() {
        let content_type = mime_type_for_format(file_ext.as_deref());

        let metadata = tokio::fs::metadata(&sanitized_disk_path)
            .await
            .map_err(|e| {
                tracing::error!("Failed to read file metadata: {}", e);
                actix_web::error::ErrorInternalServerError("Failed to read file metadata")
            })?;
        let size = metadata.len();

        let file = tokio::fs::File::open(&sanitized_disk_path)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open file: {}", e);
                actix_web::error::ErrorInternalServerError("Failed to open file")
            })?;
        let stream = ReaderStream::new(file).map_err(actix_web::error::ErrorInternalServerError);

        return Ok(HttpResponse::Ok()
            .content_type(content_type)
            .insert_header(("Content-Length", size.to_string()))
            .streaming(stream));
    }

    let image_bytes: Vec<u8>;
    if !sanitized_disk_path.exists() {
        // If the file doesn't exist, check if a fallback image URL is configured
        if config.fallback_image_url.is_some()
            && !config.fallback_image_url.as_ref().unwrap().is_empty()
        {
            let url = match &config.fallback_image_url {
                Some(base_url) => format!("{}{}", base_url, filename),
                None => filename.to_string(),
            };

            let mut upstream_response = http_client.get(&url).send().await.map_err(|e| {
                tracing::debug!("Failed to fetch fallback image: {}", e);
                // Reflect upstream fetch errors
                actix_web::error::ErrorBadGateway("Failed to fetch fallback image")
            })?;

            if !upstream_response.status().is_success() {
                tracing::debug!(
                    "Failed to fetch fallback image, status: {}",
                    upstream_response.status()
                );
                return Err(actix_web::error::InternalError::new(
                    "Failed to fetch fallback image",
                    upstream_response.status(),
                )
                .into());
            }

            if query_params.is_empty() {
                // Return a streaming response for the fallback image if no transformations are requested
                return Ok(HttpResponse::Ok()
                    .content_type(upstream_response.content_type())
                    .streaming(upstream_response));
            }

            // Otherwise, attempt to load the fallback image and apply transformations to it
            let fallback_image_body = upstream_response.body().await?;
            image_bytes = fallback_image_body.to_vec();
        } else {
            return Ok(HttpResponse::NotFound().body("File not found"));
        }
    } else {
        // Load the original file bytes from disk
        let path = sanitized_disk_path.clone();
        image_bytes = load_bytes_from_disk(&path)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read file"))?;
    }

    // Use the explicitly requested format, or fall back to the original file's format
    let effective_format = format_param.or(file_ext).unwrap_or_default();
    let content_type = mime_type_for_format(Some(effective_format.as_str()));

    // Offload all CPU-heavy image work (decode + resize + encode) to the blocking threadpool
    let config = config.get_ref().clone();
    let result_image_bytes = web::block(move || -> anyhow::Result<Vec<u8>> {
        let image = image::load_from_memory(&image_bytes)?;
        let bytes = crate::operations::pipeline::image_pipeline(
            image,
            size_param,
            &effective_format,
            &config,
        )?;
        Ok(bytes)
    })
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Blocking error"))?
    .map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to convert image: {}", e))
    })?;

    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .body(result_image_bytes))
}
