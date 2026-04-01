use image::{DynamicImage, EncodableLayout, ImageEncoder, codecs::png::CompressionType};
use webp::WebPConfig;

use crate::config::EncodingConfig;

#[tracing::instrument(level = "debug", skip_all, fields(format = ?format))]
pub fn convert_image_format(
    image: DynamicImage,
    format: Option<&str>,
    config: &EncodingConfig,
) -> anyhow::Result<Vec<u8>> {
    let mut buffer = Vec::new();

    match format {
        Some("avif") => {
            let encoder = image::codecs::avif::AvifEncoder::new_with_speed_quality(
                &mut buffer,
                config.avif_speed,
                config.avif_quality,
            );
            encoder.write_image(
                image.as_bytes(),
                image.width(),
                image.height(),
                image.color().into(),
            )?
        }
        Some("jpeg") | Some("jpg") => {
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut buffer,
                config.jpeg_quality,
            );

            let converted_to_rgb = image.into_rgb8();

            encoder.write_image(
                converted_to_rgb.as_bytes(),
                converted_to_rgb.width(),
                converted_to_rgb.height(),
                image::ExtendedColorType::Rgb8,
            )?
        }
        Some("png") => {
            let encoder = image::codecs::png::PngEncoder::new_with_quality(
                &mut buffer,
                CompressionType::Level(config.png_compression_level),
                image::codecs::png::FilterType::Adaptive,
            );
            encoder.write_image(
                image.as_bytes(),
                image.width(),
                image.height(),
                image.color().into(),
            )?
        }
        Some("webp") => {
            // Convert to RGBA once upfront; avoids the re-conversion inside from_image()
            let rgba = image.into_rgba8();
            let (w, h) = (rgba.width(), rgba.height());
            let encoder = webp::Encoder::from_rgba(rgba.as_raw(), w, h);

            let mut webp_config = WebPConfig::new()
                .map_err(|e| anyhow::anyhow!("Failed to create WebPConfig: {:?}", e))?;
            webp_config.lossless = 0;
            webp_config.alpha_compression = 1;
            webp_config.quality = config.webp_quality as f32;
            webp_config.method = config.webp_effort as i32;

            let webp_data = encoder
                .encode_advanced(&webp_config)
                .map_err(|e| anyhow::anyhow!("Failed to encode WebP image: {:?}", e))?;

            buffer.extend_from_slice(&webp_data);
        }
        Some(other) => {
            anyhow::bail!("Unsupported format: {}", other);
        }
        None => {
            return Ok(image.as_bytes().to_vec());
        }
    };

    Ok(buffer)
}
