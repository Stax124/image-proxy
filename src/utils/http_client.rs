use std::time::Duration;

/// Shared factory for the outbound `awc` client used for fallback image fetches.
///
/// Applies the given `User-Agent` as a default header and a 5s request timeout.
pub fn build_http_client(user_agent: &str) -> awc::Client {
    awc::ClientBuilder::new()
        .add_default_header((awc::http::header::USER_AGENT, user_agent))
        .timeout(Duration::from_secs(5))
        .finish()
}
