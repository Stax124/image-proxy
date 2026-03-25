use image::DynamicImage;

use crate::{
    config::EncodingConfig,
    operations::{format::convert_image_format, resize::resize_image},
};

pub fn image_pipeline(
    image: DynamicImage,
    size: Option<u32>,
    format: &str,
    config: &EncodingConfig,
) -> anyhow::Result<Vec<u8>> {
    let image = resize_image(image, size, config.use_faster_resize);
    convert_image_format(image, Some(format), config)
}
