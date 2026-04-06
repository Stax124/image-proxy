use image::DynamicImage;
use prometheus::HistogramVec;

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
    pipeline_duration: Option<&HistogramVec>,
) -> anyhow::Result<Vec<u8>> {
    let image = {
        let _timer = pipeline_duration.map(|h| h.with_label_values(&["resize"]).start_timer());
        resize_image(image, size, resize_algorithm, config)
    };
    let result = {
        let _timer = pipeline_duration.map(|h| h.with_label_values(&["encode"]).start_timer());
        convert_image_format(image, Some(format), config)
    };
    result
}
