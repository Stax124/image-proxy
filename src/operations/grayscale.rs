use image::DynamicImage;

/// Apply black-and-white (luminance grayscale) conversion when enabled.
///
/// Uses Rec. 709 / sRGB coefficients via [`DynamicImage::grayscale`]:
/// `Y = 0.2126 R + 0.7152 G + 0.0722 B`. Alpha is preserved when present.
/// Pure luma buffers are cloned unchanged; luma+alpha is re-derived with
/// alpha preserved.
#[tracing::instrument(level = "debug", skip_all, fields(enabled = enabled))]
#[hotpath::measure]
pub fn apply_bw(image: DynamicImage, enabled: bool) -> DynamicImage {
    if !enabled {
        return image;
    }
    tracing::debug!("Applying black-and-white (luminance grayscale) conversion");
    image.grayscale()
}
