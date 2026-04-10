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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EncodingConfig;

    fn test_config() -> EncodingConfig {
        EncodingConfig::default()
    }

    fn make_rgb_image(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(image::RgbaImage::from_fn(w, h, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255])
        }))
    }

    #[test]
    fn convert_to_jpeg() {
        let img = make_rgb_image(64, 64);
        let config = test_config();
        let bytes = convert_image_format(img, Some("jpeg"), &config).unwrap();
        // JPEG files start with FFD8
        assert!(bytes.len() > 2);
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0xD8);
    }

    #[test]
    fn convert_to_jpg_alias() {
        let img = make_rgb_image(64, 64);
        let config = test_config();
        let bytes = convert_image_format(img, Some("jpg"), &config).unwrap();
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0xD8);
    }

    #[test]
    fn convert_to_png() {
        let img = make_rgb_image(64, 64);
        let config = test_config();
        let bytes = convert_image_format(img, Some("png"), &config).unwrap();
        // PNG magic bytes: 89 50 4E 47
        assert!(bytes.len() > 4);
        assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn convert_to_webp() {
        let img = make_rgb_image(64, 64);
        let config = test_config();
        let bytes = convert_image_format(img, Some("webp"), &config).unwrap();
        // WebP starts with RIFF....WEBP
        assert!(bytes.len() > 12);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WEBP");
    }

    #[test]
    fn convert_to_avif() {
        let img = make_rgb_image(64, 64);
        let config = test_config();
        let bytes = convert_image_format(img, Some("avif"), &config).unwrap();
        // AVIF files contain the ftyp box with "avif" brand
        assert!(bytes.len() > 12);
        let content = String::from_utf8_lossy(&bytes);
        assert!(content.contains("ftyp") || content.contains("avif"));
    }

    #[test]
    fn convert_none_format_returns_raw_bytes() {
        let img = make_rgb_image(4, 4);
        let config = test_config();
        let bytes = convert_image_format(img.clone(), None, &config).unwrap();
        assert_eq!(bytes, img.as_bytes());
    }

    #[test]
    fn convert_unsupported_format_errors() {
        let img = make_rgb_image(4, 4);
        let config = test_config();
        let result = convert_image_format(img, Some("bmp"), &config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported format")
        );
    }

    #[test]
    fn convert_respects_jpeg_quality() {
        let img = make_rgb_image(64, 64);

        let mut low_q = test_config();
        low_q.jpeg_quality = 10;
        let bytes_low = convert_image_format(img.clone(), Some("jpeg"), &low_q).unwrap();

        let mut high_q = test_config();
        high_q.jpeg_quality = 100;
        let bytes_high = convert_image_format(img, Some("jpeg"), &high_q).unwrap();

        // Higher quality should produce larger output
        assert!(bytes_high.len() > bytes_low.len());
    }

    #[test]
    fn convert_respects_webp_quality() {
        let img = make_rgb_image(64, 64);

        let mut low_q = test_config();
        low_q.webp_quality = 1;
        let bytes_low = convert_image_format(img.clone(), Some("webp"), &low_q).unwrap();

        let mut high_q = test_config();
        high_q.webp_quality = 100;
        let bytes_high = convert_image_format(img, Some("webp"), &high_q).unwrap();

        assert!(bytes_high.len() > bytes_low.len());
    }
}
