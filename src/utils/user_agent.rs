pub const USER_AGENT_ENV: &str = "IMAGE_PROXY_USER_AGENT";

pub fn default_user_agent() -> String {
    format!("image-proxy/{}", env!("CARGO_PKG_VERSION"))
}

pub fn resolve_user_agent() -> String {
    user_agent_from_override(std::env::var(USER_AGENT_ENV).ok())
}

pub fn user_agent_from_override(override_value: Option<String>) -> String {
    match override_value {
        Some(value) if !value.trim().is_empty() => value,
        _ => default_user_agent(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_contains_crate_version() {
        let expected = format!("image-proxy/{}", env!("CARGO_PKG_VERSION"));
        assert_eq!(default_user_agent(), expected);
    }

    #[test]
    fn override_none_falls_back_to_default() {
        assert_eq!(user_agent_from_override(None), default_user_agent());
    }

    #[test]
    fn override_empty_falls_back_to_default() {
        assert_eq!(
            user_agent_from_override(Some(String::new())),
            default_user_agent()
        );
    }

    #[test]
    fn override_whitespace_falls_back_to_default() {
        assert_eq!(
            user_agent_from_override(Some("   ".to_string())),
            default_user_agent()
        );
    }

    #[test]
    fn override_custom_value_is_used_verbatim() {
        let custom = "MyCompany-ImageFetcher/2.1 (+https://example.com/bot)";
        assert_eq!(user_agent_from_override(Some(custom.to_string())), custom);
    }
}
