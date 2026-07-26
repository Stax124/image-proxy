use std::str::FromStr;

use image::DynamicImage;

/// The algorithm to use when resizing an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAlgorithm {
    /// High-quality Lanczos3 resampling.
    Lanczos3,
    /// Fast thumbnail (box/nearest) resampling.
    Thumbnail,
    /// Cubic (Catmull-Rom) resampling — good quality/speed tradeoff vs Lanczos3.
    Bicubic,
    /// Gaussian resampling — smoother result, less ringing than Lanczos3.
    Gaussian,
    /// Choose automatically: use `Thumbnail` for large downscales (< 50 % of
    /// the original longest edge), otherwise `Lanczos3`.
    Auto,
}

impl FromStr for ResizeAlgorithm {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "lanczos3" => Ok(Self::Lanczos3),
            "thumbnail" => Ok(Self::Thumbnail),
            "bicubic" => Ok(Self::Bicubic),
            "gaussian" => Ok(Self::Gaussian),
            "auto" => Ok(Self::Auto),
            _ => Err(()),
        }
    }
}

#[tracing::instrument(level = "debug", skip_all, fields(size = ?size, ?algorithm))]
#[hotpath::measure]
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
        // Use the fast thumbnail path when scaling to less than 50 % of the
        // original longest edge; prefer Lanczos3 for minor size reductions.
        let original_max = image.width().max(image.height()) as f64;
        if (size as f64 / original_max) < 0.5 {
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

    // Resize the image while maintaining aspect ratio.
    // Auto is resolved above; Thumbnail has no FilterType and uses a separate path.
    match filter_type(algorithm) {
        Some(filter) => {
            let (new_width, new_height) = target_dimensions(image.width(), image.height(), size);
            image.resize_exact(new_width, new_height, filter);
            image
        }
        None => image.thumbnail(size, size),
    }
}

/// Map algorithms that use `resize_exact` to a filter; `Thumbnail`/`Auto` have none.
fn filter_type(algorithm: ResizeAlgorithm) -> Option<image::imageops::FilterType> {
    match algorithm {
        ResizeAlgorithm::Lanczos3 => Some(image::imageops::FilterType::Lanczos3),
        ResizeAlgorithm::Bicubic => Some(image::imageops::FilterType::CatmullRom),
        ResizeAlgorithm::Gaussian => Some(image::imageops::FilterType::Gaussian),
        ResizeAlgorithm::Thumbnail | ResizeAlgorithm::Auto => None,
    }
}

/// Fit `size` as the longest edge while preserving aspect ratio.
fn target_dimensions(width: u32, height: u32, size: u32) -> (u32, u32) {
    let aspect_ratio = width as f64 / height as f64;
    if aspect_ratio > 1.0 {
        // Landscape orientation
        (size, (size as f64 / aspect_ratio).round() as u32)
    } else {
        // Portrait orientation (and square)
        ((size as f64 * aspect_ratio).round() as u32, size)
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
            Ok(ResizeAlgorithm::Lanczos3)
        );
    }

    #[test]
    fn from_str_thumbnail() {
        assert_eq!(
            ResizeAlgorithm::from_str("thumbnail"),
            Ok(ResizeAlgorithm::Thumbnail)
        );
    }

    #[test]
    fn from_str_auto() {
        assert_eq!(ResizeAlgorithm::from_str("auto"), Ok(ResizeAlgorithm::Auto));
    }

    #[test]
    fn from_str_bicubic() {
        assert_eq!(
            ResizeAlgorithm::from_str("bicubic"),
            Ok(ResizeAlgorithm::Bicubic)
        );
    }

    #[test]
    fn from_str_gaussian() {
        assert_eq!(
            ResizeAlgorithm::from_str("gaussian"),
            Ok(ResizeAlgorithm::Gaussian)
        );
    }

    #[test]
    fn from_str_case_insensitive() {
        assert_eq!(
            ResizeAlgorithm::from_str("LANCZOS3"),
            Ok(ResizeAlgorithm::Lanczos3)
        );
        assert_eq!(
            ResizeAlgorithm::from_str("Thumbnail"),
            Ok(ResizeAlgorithm::Thumbnail)
        );
        assert_eq!(
            ResizeAlgorithm::from_str("Bicubic"),
            Ok(ResizeAlgorithm::Bicubic)
        );
        assert_eq!(
            ResizeAlgorithm::from_str("Gaussian"),
            Ok(ResizeAlgorithm::Gaussian)
        );
        assert_eq!(ResizeAlgorithm::from_str("AUTO"), Ok(ResizeAlgorithm::Auto));
    }

    #[test]
    fn from_str_invalid() {
        assert_eq!(ResizeAlgorithm::from_str(""), Err(()));
        assert_eq!(ResizeAlgorithm::from_str("bilinear"), Err(()));
        assert_eq!(ResizeAlgorithm::from_str("nearest"), Err(()));
    }

    // --- target_dimensions ---

    #[test]
    fn target_dimensions_landscape() {
        assert_eq!(target_dimensions(1000, 500, 400), (400, 200));
    }

    #[test]
    fn target_dimensions_portrait() {
        assert_eq!(target_dimensions(500, 1000, 400), (200, 400));
    }

    #[test]
    fn target_dimensions_square() {
        assert_eq!(target_dimensions(800, 800, 200), (200, 200));
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

    #[test]
    fn resize_bicubic_landscape() {
        let img = make_image(1000, 500);
        let config = test_config();
        let result = resize_image(img, Some(400), Some(ResizeAlgorithm::Bicubic), &config);

        assert_eq!(result.width(), 400);
        assert_eq!(result.height(), 200);
    }

    #[test]
    fn resize_bicubic_portrait() {
        let img = make_image(500, 1000);
        let config = test_config();
        let result = resize_image(img, Some(400), Some(ResizeAlgorithm::Bicubic), &config);

        assert_eq!(result.height(), 400);
        assert_eq!(result.width(), 200);
    }

    #[test]
    fn resize_gaussian_landscape() {
        let img = make_image(1000, 500);
        let config = test_config();
        let result = resize_image(img, Some(400), Some(ResizeAlgorithm::Gaussian), &config);

        assert_eq!(result.width(), 400);
        assert_eq!(result.height(), 200);
    }

    #[test]
    fn resize_gaussian_portrait() {
        let img = make_image(500, 1000);
        let config = test_config();
        let result = resize_image(img, Some(400), Some(ResizeAlgorithm::Gaussian), &config);

        assert_eq!(result.height(), 400);
        assert_eq!(result.width(), 200);
    }
}
