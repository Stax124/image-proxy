use image::DynamicImage;

#[tracing::instrument(level = "debug", skip_all, fields(len = bytes.len()))]
#[hotpath::measure]
pub fn decode_image(bytes: &[u8]) -> anyhow::Result<DynamicImage> {
    Ok(image::load_from_memory(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_valid_jpeg() {
        // Synthesize a small valid JPEG using the image crate
        let img = DynamicImage::new_rgb8(4, 4);
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 80);
        image::ImageEncoder::write_image(
            encoder,
            img.as_bytes(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();

        let decoded = decode_image(&buf).unwrap();
        assert_eq!(decoded.width(), 4);
        assert_eq!(decoded.height(), 4);
    }

    #[test]
    fn decode_invalid_data_errors() {
        let result = decode_image(b"not an image");
        assert!(result.is_err());
    }
}
