use std::sync::Arc;

use crate::{
    config::EncodingConfig,
    operations::resize::ResizeAlgorithm,
    preferred_formats::get_preferred_format,
    utils::{
        PathValidationError, load_bytes_from_disk, mime_type_for_format, sanitize_and_validate_path,
    },
};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use bytes::Bytes;
use foyer::HybridCache;
use futures_util::TryStreamExt;
use prometheus::{HistogramVec, IntCounterVec};
use tokio_util::io::ReaderStream;

const SUPPORTED_INPUT_FORMATS: &[&str] = &["avif", "jpeg", "jpg", "png", "webp"];

pub fn add_headers_for_caching(
    response: &mut actix_web::HttpResponseBuilder,
    config: &EncodingConfig,
) {
    response.insert_header(("Vary", "Accept, Sec-CH-DPR"));
    if !config.cache_control_header.is_empty() {
        response.insert_header(("Cache-Control", config.cache_control_header.clone()));
    }
}

#[actix_web::get("/{filename:.*}")]
#[tracing::instrument(skip_all, fields(filename = %filename), level = "debug")]
pub async fn process_image_request(
    req: HttpRequest,
    filename: web::Path<String>,
    query_params: web::Query<std::collections::HashMap<String, String>>,
    config: web::Data<Arc<EncodingConfig>>,
    http_client: web::Data<awc::Client>,
    cache: web::Data<Option<HybridCache<String, Bytes>>>,
    pipeline_duration: web::Data<HistogramVec>,
    request_count: web::Data<IntCounterVec>,
) -> actix_web::Result<HttpResponse> {
    let (sanitized_path, sanitized_disk_path, ext) = match sanitize_and_validate_path(
        &filename,
        config.strip_path.as_deref(),
        &config.root_path,
        SUPPORTED_INPUT_FORMATS,
    ) {
        Ok(result) => result,
        Err(PathValidationError::MissingExtension) => {
            request_count
                .with_label_values(&["unknown", "unsupported_media_type"])
                .inc();
            return Ok(
                HttpResponse::UnsupportedMediaType().body("Missing or unsupported file format")
            );
        }
        Err(PathValidationError::UnsupportedFormat(ext)) => {
            request_count
                .with_label_values(&[ext.as_str(), "unsupported_media_type"])
                .inc();
            return Ok(HttpResponse::UnsupportedMediaType()
                .body(format!("Unsupported file format: {}", ext)));
        }
    };
    let file_ext = Some(ext.clone());

    // Special headers that can influence processing
    let sec_ch_dpr = req
        .headers()
        .get("Sec-CH-DPR")
        .and_then(|v| v.to_str().ok());

    tracing::debug!(
        query_params = ?query_params,
        strip_path = ?config.strip_path,
        root_path = ?config.root_path,
        sanitized_path = ?sanitized_path,
        sanitized_disk_path = ?sanitized_disk_path,
        filename = ?filename,
        sec_ch_dpr = ?sec_ch_dpr
    );

    let format_param = query_params.get("format").map(|s| s.to_lowercase());
    let dpr_param = query_params
        .get("dpr")
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| sec_ch_dpr.and_then(|dpr_str| dpr_str.parse::<f64>().ok()))
        .filter(|&d| (1.0..=10.0).contains(&d));
    let size_param = query_params
        .get("size")
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|&s| s > 0)
        .map(|s| {
            if let Some(dpr) = dpr_param {
                (s as f64 * dpr).round() as u32
            } else {
                s
            }
        });
    let resize_algorithm_param = query_params
        .get("resize_algorithm")
        .and_then(|s| ResizeAlgorithm::from_str(s));

    // If user requested an explicit output format, check if it's allowed by configuration, if not, reject the request early before doing any expensive work
    if let Some(fmt) = format_param.as_ref()
        && let Some(allowed_formats) = &config.allowed_output_formats
    {
        if !allowed_formats.iter().any(|f| f.eq_ignore_ascii_case(fmt)) {
            request_count
                .with_label_values(&[fmt.as_str(), "unsupported_media_type"])
                .inc();
            return Ok(HttpResponse::UnsupportedMediaType()
                .body(format!("Requested output format '{}' is not allowed", fmt)));
        }
    }

    // Use the explicitly requested format, or fall back to the original file's format
    let effective_format = get_preferred_format(
        &config,
        format_param,
        file_ext.as_deref().unwrap_or(""),
        req.headers()
            .get("Accept")
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
        false,
    );
    let content_type = mime_type_for_format(Some(effective_format.as_str()));

    // Build a cache key from the path and transformation parameters
    let cache_key = format!(
        "{}?format={}&size={}&resize={}&dpr={}",
        sanitized_path.display(),
        effective_format,
        size_param.map_or_else(|| "none".to_string(), |s| s.to_string()),
        resize_algorithm_param.map_or_else(|| "none".to_string(), |r| format!("{:?}", r)),
        dpr_param.map_or_else(|| "none".to_string(), |d| format!("{:.2}", d)),
    );

    // Check cache for a hit before doing any expensive work
    if let Some(cache) = cache.get_ref() {
        if let Ok(Some(entry)) = cache.get(&cache_key).await {
            tracing::debug!(cache_key = %cache_key, "cache hit");
            request_count
                .with_label_values(&[effective_format.as_str(), "ok"])
                .inc();
            let mut builder = HttpResponse::Ok();
            builder.content_type(content_type);
            builder.insert_header(("X-Cache", "HIT"));
            add_headers_for_caching(&mut builder, &config);
            return Ok(builder.body(entry.value().clone()));
        }
    }

    // Stream local files directly when no transformation is requested.
    if query_params.is_empty()
        && sanitized_disk_path.exists()
        && file_ext.as_deref() == Some(effective_format.as_str())
    {
        let content_type = mime_type_for_format(file_ext.as_deref());

        let metadata = tokio::fs::metadata(&sanitized_disk_path)
            .await
            .map_err(|e| {
                tracing::error!("Failed to read file metadata: {}", e);
                request_count
                    .with_label_values(&[ext.as_str(), "error"])
                    .inc();
                actix_web::error::ErrorInternalServerError("Failed to read file metadata")
            })?;
        let size = metadata.len();

        let file = tokio::fs::File::open(&sanitized_disk_path)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open file: {}", e);
                request_count
                    .with_label_values(&[ext.as_str(), "error"])
                    .inc();
                actix_web::error::ErrorInternalServerError("Failed to open file")
            })?;
        let stream = ReaderStream::new(file).map_err(actix_web::error::ErrorInternalServerError);

        request_count.with_label_values(&[ext.as_str(), "ok"]).inc();
        let mut builder = HttpResponse::Ok();
        builder.content_type(content_type);
        builder.insert_header(("Content-Length", size.to_string()));
        add_headers_for_caching(&mut builder, &config);
        return Ok(builder.streaming(stream));
    }

    let image_bytes: Bytes;
    if !sanitized_disk_path.exists() {
        // If the file doesn't exist, check if a fallback image URL is configured
        if config.fallback_image_url.is_some()
            && !config.fallback_image_url.as_ref().unwrap().is_empty()
        {
            let url = match &config.fallback_image_url {
                Some(base_url) => format!("{}{}", base_url, sanitized_path.display()),
                None => sanitized_path.display().to_string(),
            };

            let mut upstream_response = http_client.get(&url).send().await.map_err(|e| {
                tracing::debug!("Failed to fetch fallback image: {}", e);
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

            let content_type = upstream_response.content_type();
            let parsed_format = content_type
                .split('/')
                .nth(1)
                .and_then(|s| s.split(';').next())
                .map(|s| s.trim().to_lowercase())
                .unwrap_or_else(|| "".to_string());

            // If the upstream fallback image is in the same format as the requested format and no transformations
            // are requested, we can directly stream it without loading it into memory
            if query_params.is_empty() && parsed_format == effective_format {
                // Return a streaming response for the fallback image if no transformations are requested
                let mut builder = HttpResponse::Ok();
                builder.content_type(content_type);
                add_headers_for_caching(&mut builder, &config);
                return Ok(builder.streaming(upstream_response));
            }

            // Otherwise, attempt to load the fallback image and apply transformations to it
            image_bytes = upstream_response
                .body()
                .limit(config.fallback_image_max_size)
                .await?;
        } else {
            request_count
                .with_label_values(&[ext.as_str(), "not_found"])
                .inc();
            return Ok(HttpResponse::NotFound().body("File not found"));
        }
    } else {
        // Load the original file bytes from disk
        let path = sanitized_disk_path.clone();
        image_bytes = load_bytes_from_disk(&path)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read file"))?
            .into();
    }

    // Offload all CPU-heavy image work (decode + resize + encode) to the blocking threadpool
    let config_for_pipeline = config.get_ref().clone();
    let pipeline_duration = pipeline_duration.get_ref().clone();
    let effective_format_clone = effective_format.clone();
    let result_image_bytes = web::block(move || -> anyhow::Result<Vec<u8>> {
        let image = {
            let _timer = pipeline_duration
                .with_label_values(&["decode"])
                .start_timer();
            image::load_from_memory(&image_bytes)?
        };
        let bytes = crate::operations::pipeline::image_pipeline(
            image,
            size_param,
            &effective_format_clone,
            &config_for_pipeline,
            resize_algorithm_param,
            Some(&pipeline_duration),
        )?;
        Ok(bytes)
    })
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Blocking error"))?
    .map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to convert image: {}", e))
    })?;

    // Store the transformed result in cache
    let result_image_bytes = Bytes::from(result_image_bytes);
    if let Some(cache) = cache.get_ref() {
        cache.insert(cache_key, result_image_bytes.clone());
    }

    request_count
        .with_label_values(&[effective_format.as_str(), "ok"])
        .inc();
    let mut builder = HttpResponse::Ok();
    builder.content_type(content_type);
    builder.insert_header(("X-Cache", "MISS"));
    add_headers_for_caching(&mut builder, &config);
    Ok(builder.body(result_image_bytes))
}
