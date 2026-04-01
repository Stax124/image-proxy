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
