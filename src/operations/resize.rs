use image::DynamicImage;

/// The algorithm to use when resizing an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAlgorithm {
    /// High-quality Lanczos3 resampling.
    Lanczos3,
    /// Fast thumbnail (box/nearest) resampling.
    Thumbnail,
    /// Choose automatically: use `Thumbnail` for large downscales (< 80 % of
    /// the original longest edge), otherwise `Lanczos3`.
    Auto,
}

impl ResizeAlgorithm {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "lanczos3" => Some(Self::Lanczos3),
            "thumbnail" => Some(Self::Thumbnail),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

#[tracing::instrument(level = "debug", skip_all, fields(size = ?size, ?algorithm))]
pub fn resize_image(
    mut image: DynamicImage,
    size: Option<u32>,
    algorithm: Option<ResizeAlgorithm>,
    config: &crate::config::EncodingConfig,
) -> DynamicImage {
    // If size is not specified or zero, return the original image
    let size = size.unwrap_or(0);
    if size == 0 {
        return image;
    }

    // Use the specified algorithm or fall back to the default from the config
    let algorithm = algorithm.unwrap_or(config.resize_algorithm);

    // Do not go larger than the original image size
    let max_height = image.height().min(size);
    let max_width = image.width().min(size);
    let size = max_height.max(max_width);

    let algorithm = if algorithm == ResizeAlgorithm::Auto {
        // Use the fast thumbnail path when scaling to less than 80 % of the
        // original longest edge; prefer Lanczos3 for minor size reductions.
        let original_max = image.width().max(image.height()) as f64;
        if (size as f64 / original_max) < 0.8 {
            ResizeAlgorithm::Thumbnail
        } else {
            ResizeAlgorithm::Lanczos3
        }
    } else {
        algorithm
    };

    tracing::debug!(
        "Resizing image to size {} using algorithm {:?}",
        size,
        algorithm
    );

    // Resize the image while maintaining aspect ratio
    match algorithm {
        ResizeAlgorithm::Lanczos3 => {
            let aspect_ratio = image.width() as f64 / image.height() as f64;
            let (new_width, new_height) = if aspect_ratio > 1.0 {
                // Landscape orientation
                (size, (size as f64 / aspect_ratio).round() as u32)
            } else {
                // Portrait orientation
                ((size as f64 * aspect_ratio).round() as u32, size)
            };

            image.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3);
            image
        }
        ResizeAlgorithm::Thumbnail => image.thumbnail(size, size),
        ResizeAlgorithm::Auto => unreachable!(), // Already handled above
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EncodingConfig;

    fn test_config() -> EncodingConfig {
        EncodingConfig::default()
    }

    fn make_image(w: u32, h: u32) -> DynamicImage {
        DynamicImage::new_rgba8(w, h)
    }

    // --- ResizeAlgorithm::from_str ---

    #[test]
    fn from_str_lanczos3() {
        assert_eq!(
            ResizeAlgorithm::from_str("lanczos3"),
            Some(ResizeAlgorithm::Lanczos3)
        );
    }

    #[test]
    fn from_str_thumbnail() {
        assert_eq!(
            ResizeAlgorithm::from_str("thumbnail"),
            Some(ResizeAlgorithm::Thumbnail)
        );
    }

    #[test]
    fn from_str_auto() {
        assert_eq!(
            ResizeAlgorithm::from_str("auto"),
            Some(ResizeAlgorithm::Auto)
        );
    }

    #[test]
    fn from_str_case_insensitive() {
        assert_eq!(
            ResizeAlgorithm::from_str("LANCZOS3"),
            Some(ResizeAlgorithm::Lanczos3)
        );
        assert_eq!(
            ResizeAlgorithm::from_str("Thumbnail"),
            Some(ResizeAlgorithm::Thumbnail)
        );
        assert_eq!(
            ResizeAlgorithm::from_str("AUTO"),
            Some(ResizeAlgorithm::Auto)
        );
    }

    #[test]
    fn from_str_invalid() {
        assert_eq!(ResizeAlgorithm::from_str(""), None);
        assert_eq!(ResizeAlgorithm::from_str("bilinear"), None);
        assert_eq!(ResizeAlgorithm::from_str("nearest"), None);
    }

    // --- resize_image ---

    #[test]
    fn resize_no_size_returns_original() {
        let img = make_image(100, 200);
        let config = test_config();
        let result = resize_image(img, None, None, &config);
        assert_eq!(result.width(), 100);
        assert_eq!(result.height(), 200);
    }

    #[test]
    fn resize_zero_size_returns_original() {
        let img = make_image(100, 200);
        let config = test_config();
        let result = resize_image(img, Some(0), None, &config);
        assert_eq!(result.width(), 100);
        assert_eq!(result.height(), 200);
    }

    #[test]
    fn resize_does_not_upscale() {
        let img = make_image(50, 50);
        let config = test_config();
        let result = resize_image(img, Some(200), Some(ResizeAlgorithm::Thumbnail), &config);

        assert_eq!(result.width(), 50);
        assert_eq!(result.height(), 50);
    }

    #[test]
    fn resize_thumbnail_downscale() {
        let img = make_image(1000, 500);
        let config = test_config();
        let result = resize_image(img, Some(200), Some(ResizeAlgorithm::Thumbnail), &config);

        assert_eq!(result.width(), 200);
        assert_eq!(result.height(), 100);
    }

    #[test]
    fn resize_lanczos3_landscape() {
        let img = make_image(1000, 500);
        let config = test_config();
        let result = resize_image(img, Some(400), Some(ResizeAlgorithm::Lanczos3), &config);

        assert_eq!(result.width(), 400);
        assert_eq!(result.height(), 200);
    }

    #[test]
    fn resize_lanczos3_portrait() {
        let img = make_image(500, 1000);
        let config = test_config();
        let result = resize_image(img, Some(400), Some(ResizeAlgorithm::Lanczos3), &config);

        assert_eq!(result.height(), 400);
        assert_eq!(result.width(), 200);
    }
}
