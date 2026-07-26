use image::DynamicImage;
use prometheus::HistogramVec;

use crate::{
    config::EncodingConfig,
    operations::{
        format::convert_image_format,
        grayscale::apply_bw,
        resize::{ResizeAlgorithm, resize_image},
    },
};

#[tracing::instrument(level = "debug", skip_all)]
#[hotpath::measure]
pub fn image_pipeline(
    image: DynamicImage,
    size: Option<u32>,
    format: &str,
    config: &EncodingConfig,
    resize_algorithm: Option<ResizeAlgorithm>,
    black_and_white: bool,
    pipeline_duration: Option<&HistogramVec>,
) -> anyhow::Result<Vec<u8>> {
    // Labels: step, format. Format is meaningful for encode (and kept consistent for all steps).
    let image = {
        let _timer =
            pipeline_duration.map(|h| h.with_label_values(&["resize", format]).start_timer());
        resize_image(image, size, resize_algorithm, config)
    };

    let image = if black_and_white {
        let _timer = pipeline_duration.map(|h| h.with_label_values(&["bw", format]).start_timer());
        apply_bw(image, true)
    } else {
        image
    };

    let _timer = pipeline_duration.map(|h| h.with_label_values(&["encode", format]).start_timer());
    convert_image_format(image, Some(format), config)
}
