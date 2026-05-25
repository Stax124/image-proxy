pub fn mime_type_for_format(format: Option<&str>) -> &'static str {
    match format {
        Some("avif") => "image/avif",
        Some("jpeg") | Some("jpg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("jxl") => "image/jxl",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("tiff") => "image/tiff",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_type_avif() {
        assert_eq!(mime_type_for_format(Some("avif")), "image/avif");
    }

    #[test]
    fn mime_type_unknown_format() {
        assert_eq!(
            mime_type_for_format(Some("xyz")),
            "application/octet-stream"
        );
    }

    #[test]
    fn mime_type_none() {
        assert_eq!(mime_type_for_format(None), "application/octet-stream");
    }
}
