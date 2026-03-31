use image::DynamicImage;

#[tracing::instrument(level = "debug", skip_all, fields(size = ?size, use_faster_resize))]
pub fn resize_image(
    mut image: DynamicImage,
    size: Option<u32>,
    use_faster_resize: bool,
) -> DynamicImage {
    // If size is not specified or zero, return the original image
    let size = size.unwrap_or(0);
    if size == 0 {
        return image;
    }

    // Do not go larger than the original image size
    let max_height = image.height().min(size);
    let max_width = image.width().min(size);
    let size = max_height.max(max_width);

    // Resize the image while maintaining aspect ratio
    if use_faster_resize {
        image.thumbnail(size, size)
    } else {
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
}
