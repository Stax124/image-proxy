use std::path::Path;
use std::sync::Arc;

use crate::{
    config::EncodingConfig,
    utils::{convert_bytes_to_readable_size, load_bytes_from_disk, mime_type_for_format},
};
use actix_web::{HttpResponse, web};

const SUPPORTED_FORMATS: &[&str] = &["avif", "jpeg", "jpg", "png", "webp"];

#[actix_web::get("/{filename:.*}")]
#[tracing::instrument(skip_all, fields(filename = %filename), level = "debug")]
pub async fn process_image_request(
    filename: web::Path<String>,
    query_params: web::Query<std::collections::HashMap<String, String>>,
    config: web::Data<Arc<EncodingConfig>>,
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
        .fold(Path::new(&config.root_path).to_path_buf(), |acc, comp| {
            acc.join(comp.as_os_str())
        });

    // Check if the extension is valid and supported
    let file_ext = sanitized_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());
    if let Some(ext) = &file_ext {
        if !SUPPORTED_FORMATS.contains(&ext.as_str()) {
            return Ok(HttpResponse::UnsupportedMediaType()
                .body(format!("Unsupported file format: {}", ext)));
        }
    }

    tracing::debug!(
        query_params = ?query_params,
        strip_path = ?config.strip_path,
        root_path = ?config.root_path,
        sanitized_path = ?sanitized_path,
        filename = ?filename,
    );

    let file_ext = sanitized_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());

    let format_param = query_params.get("format").map(|s| s.to_lowercase());
    let size_param = query_params
        .get("size")
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|&s| s > 0);

    let image_bytes: Vec<u8>;
    if !sanitized_path.exists() {
        if config.fallback_image_url.is_some() {
            image_bytes = crate::utils::load_fallback_image_from_url(&filename, &config)
                .await
                .map_err(|t| {
                    actix_web::error::ErrorInternalServerError(format!(
                        "Failed to load fallback image: {}",
                        t
                    ))
                })?;
        } else {
            return Ok(HttpResponse::NotFound().body("File not found"));
        }
    } else {
        let path = sanitized_path.clone();
        image_bytes = web::block(move || load_bytes_from_disk(&path))
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Blocking error"))?
            .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read file"))?;
    }

    // If no transformation requested, serve the original file bytes directly
    if format_param.is_none() && size_param.is_none() {
        let content_type = mime_type_for_format(file_ext.as_deref());
        let size_str = convert_bytes_to_readable_size(image_bytes.len() as u64);
        return Ok(HttpResponse::Ok()
            .content_type(content_type)
            .insert_header(("X-Original-Size", size_str.clone()))
            .insert_header(("X-Final-Size", size_str))
            .body(image_bytes));
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
