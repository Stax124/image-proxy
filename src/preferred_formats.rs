/// Formats that properly compress alpha and should be preferred when the source image has an alpha channel
const ALPHA_PREFERRED_FORMATS: &[&str] = &["jxl", "webp"];

/// Determines the preferred output format based on the configuration, preselected format, file extension, browser support, and alpha channel presence
pub fn get_preferred_format(
    config: &crate::config::EncodingConfig,
    preselected_format: Option<String>,
    file_extension: &str,
    accept_header: &str,
    has_alpha_channel: bool,
) -> String {
    // To simplyfy the logic inside the API endpoint
    if let Some(preselected) = preselected_format {
        return preselected;
    }

    let preferred_formats = config.preferred_formats.as_deref().unwrap_or_default();
    if preferred_formats.is_empty() {
        return file_extension.to_string();
    }

    let allowed_output_formats = config.allowed_output_formats.as_deref();

    // Check if a format passes both allowed and browser-support filters without allocating
    let is_available = |format: &str| -> bool {
        if let Some(allowed) = allowed_output_formats {
            if !allowed.iter().any(|a| a == format) {
                return false;
            }
        }
        extract_preferred_formats_from_accept_header(accept_header)
            .iter()
            .any(|b| b == format)
    };

    // If the source image has an alpha channel, prefer formats that properly compress alpha
    if has_alpha_channel {
        for &format in ALPHA_PREFERRED_FORMATS {
            if preferred_formats.iter().any(|f| f == format) && is_available(format) {
                return format.to_string();
            }
        }
    }

    // Otherwise, return the first preferred format that passes filters
    preferred_formats
        .iter()
        .find(|f| is_available(f))
        .map(|f| f.to_string())
        .unwrap_or_else(|| file_extension.to_string())
}

/// Parses the Accept header to extract a list of preferred image formats in order of preference
pub fn extract_preferred_formats_from_accept_header(accept_header: &str) -> Vec<String> {
    // Should be in these formats:
    // We need to extract just the format names (e.g., "avif", "webp", "jxl") and ignore the quality values and other parameters
    accept_header
        .split(',')
        .filter_map(|part| {
            let mime_type = part.split(';').next()?.trim();
            match mime_type.strip_prefix("image/") {
                Some("*") | None => None,
                Some(format) => Some(format.to_string()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EncodingConfig;

    #[test]
    fn test_get_preferred_format_preselected() {
        let config = EncodingConfig {
            preferred_formats: Some(vec!["webp".to_string()]),
            allowed_output_formats: None,
            ..Default::default() // Assuming Default impl exists; adjust if needed
        };
        let result = get_preferred_format(
            &config,
            Some("avif".to_string()),
            "png",
            "image/avif,image/png",
            false,
        );
        assert_eq!(result, "avif");
    }

    #[test]
    fn test_get_preferred_format_empty_preferred() {
        let config = EncodingConfig {
            preferred_formats: Some(vec![]),
            ..Default::default()
        };
        let result = get_preferred_format(&config, None, "png", "image/avif,image/png", false);
        assert_eq!(result, "png");
    }

    #[test]
    fn test_get_preferred_format_filtered_by_allowed() {
        let config = EncodingConfig {
            preferred_formats: Some(vec!["jxl".to_string(), "webp".to_string()]),
            allowed_output_formats: Some(vec!["webp".to_string()]),
            ..Default::default()
        };
        let result = get_preferred_format(&config, None, "png", "image/webp", false);
        assert_eq!(result, "webp");
    }

    #[test]
    fn test_get_preferred_format_filtered_by_browser() {
        let config = EncodingConfig {
            preferred_formats: Some(vec!["jxl".to_string(), "avif".to_string()]),
            ..Default::default()
        };
        let result = get_preferred_format(&config, None, "png", "image/avif", false);
        assert_eq!(result, "avif");
    }

    #[test]
    fn test_get_preferred_format_alpha_preferred() {
        let config = EncodingConfig {
            preferred_formats: Some(vec![
                "png".to_string(),
                "jxl".to_string(),
                "webp".to_string(),
            ]),
            ..Default::default()
        };
        let result =
            get_preferred_format(&config, None, "png", "image/png,image/jxl,image/webp", true);
        assert_eq!(result, "jxl"); // First in ALPHA_PREFERRED_FORMATS that matches
    }

    #[test]
    fn test_get_preferred_format_alpha_no_match_fallback() {
        let config = EncodingConfig {
            preferred_formats: Some(vec!["png".to_string(), "avif".to_string()]),
            ..Default::default()
        };
        let result = get_preferred_format(&config, None, "png", "image/png,image/avif", true);
        assert_eq!(result, "png"); // No alpha-preferred match, first preferred
    }

    #[test]
    fn test_extract_preferred_formats_from_accept_header_chrome() {
        let accept = "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8";
        let result = extract_preferred_formats_from_accept_header(accept);
        assert_eq!(result, vec!["avif", "webp", "apng", "svg+xml"]);
    }

    #[test]
    fn test_extract_preferred_formats_from_accept_header_safari() {
        let accept = "image/webp,image/avif,image/jxl,image/heic,image/heic-sequence,video/*;q=0.8,image/png,image/svg+xml,image/*;q=0.8,*/*;q=0.5";
        let result = extract_preferred_formats_from_accept_header(accept);
        assert_eq!(
            result,
            vec![
                "webp",
                "avif",
                "jxl",
                "heic",
                "heic-sequence",
                "png",
                "svg+xml"
            ]
        );
    }

    #[test]
    fn test_extract_preferred_formats_empty() {
        let accept = "";
        let result = extract_preferred_formats_from_accept_header(accept);
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_preferred_formats_no_image() {
        let accept = "text/html,application/json,*/*;q=0.5";
        let result = extract_preferred_formats_from_accept_header(accept);
        assert!(result.is_empty());
    }
}
