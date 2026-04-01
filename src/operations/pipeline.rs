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
    let image = resize_image(image, size, resize_algorithm, config);
    convert_image_format(image, Some(format), config)
}
