use std::path::Path;

pub fn mime_type_for_format(format: Option<&str>) -> &'static str {
    match format {
        Some("avif") => "image/avif",
        Some("jpeg") | Some("jpg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}
pub fn convert_bytes_to_readable_size(bytes: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < units.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, units[unit_index])
}

pub fn load_image_from_path(path: &Path) -> anyhow::Result<image::DynamicImage> {
    let image = image::open(path)?;
    Ok(image)
}
