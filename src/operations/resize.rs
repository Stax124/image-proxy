use image::DynamicImage;

/// Resize the image in a way that preserves the aspect ratio and fits within a square of the given size.
pub fn resize_image(
    image: DynamicImage,
    size: Option<u32>,
    use_faster_resize: bool,
) -> DynamicImage {
    let size = size.unwrap_or(0);
    if size == 0 {
        return image;
    }

    if use_faster_resize {
        let aspect_ratio = image.width() as f64 / image.height() as f64;
        let (new_width, new_height) = if aspect_ratio > 1.0 {
            // Landscape orientation
            (size, (size as f64 / aspect_ratio).round() as u32)
        } else {
            // Portrait orientation
            ((size as f64 * aspect_ratio).round() as u32, size)
        };

        image.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3)
    } else {
        image.thumbnail(size, size)
    }
}
