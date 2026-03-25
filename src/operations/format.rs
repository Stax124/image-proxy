use image::{DynamicImage, ImageEncoder};

use crate::config::EncodingConfig;

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
            encoder.write_image(
                image.as_bytes(),
                image.width(),
                image.height(),
                image.color().into(),
            )?
        }
        Some("png") => {
            let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
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
            let webp_data = encoder.encode(config.webp_quality);
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
