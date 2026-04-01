use image::DynamicImage;

use crate::{
    config::EncodingConfig,
    operations::{
        format::convert_image_format,
        resize::{ResizeAlgorithm, resize_image},
    },
};

#[tracing::instrument(level = "debug", skip_all)]
pub fn image_pipeline(
    image: DynamicImage,
    size: Option<u32>,
    format: &str,
    config: &EncodingConfig,
    resize_algorithm: Option<ResizeAlgorithm>,
) -> anyhow::Result<Vec<u8>> {
    // Fall back to the global config default when no per-request algorithm is given.
    let algorithm = resize_algorithm.unwrap_or_else(|| {
        if config.use_faster_resize {
            ResizeAlgorithm::Thumbnail
        } else {
            ResizeAlgorithm::Lanczos3
        }
    });
    let image = resize_image(image, size, algorithm);
    convert_image_format(image, Some(format), config)
}
