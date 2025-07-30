use axum::{
    http::{HeaderMap, HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use std::str::FromStr;
use tracing::Span;
use uuid::Uuid;

/// Standard correlation headers
pub const CORRELATION_ID_HEADER: &str = "x-correlation-id";
pub const REQUEST_ID_HEADER: &str = "x-request-id";
pub const SESSION_ID_HEADER: &str = "x-session-id";
pub const USER_ID_HEADER: &str = "x-user-id";

/// Correlation context that gets attached to all logs and traces
#[derive(Debug, Clone)]
pub struct CorrelationContext {
    pub correlation_id: String,
    pub request_id: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub user_agent: Option<String>,
    pub remote_ip: Option<String>,
    pub forwarded_for: Option<String>,
}

impl CorrelationContext {
    /// Extract correlation context from HTTP headers
    pub fn from_headers(headers: &HeaderMap) -> Self {
        let correlation_id = extract_header_or_generate(headers, CORRELATION_ID_HEADER);
        let request_id = extract_header_or_generate(headers, REQUEST_ID_HEADER);

        let session_id = extract_optional_header(headers, SESSION_ID_HEADER);
        let user_id = extract_optional_header(headers, USER_ID_HEADER);
        let user_agent = extract_optional_header(headers, "user-agent");

        // Extract IP information (prioritize Cloudflare headers)
        let remote_ip = extract_optional_header(headers, "cf-connecting-ip")
            .or_else(|| extract_optional_header(headers, "x-real-ip"))
            .or_else(|| extract_optional_header(headers, "x-client-ip"));

        let forwarded_for = extract_optional_header(headers, "x-forwarded-for");

        Self {
            correlation_id,
            request_id,
            session_id,
            user_id,
            user_agent,
            remote_ip,
            forwarded_for,
        }
    }

    /// Add correlation context to the current tracing span
    pub fn add_to_span(&self, span: &Span) {
        span.record("correlation_id", &self.correlation_id);
        span.record("request_id", &self.request_id);

        if let Some(session_id) = &self.session_id {
            span.record("session_id", session_id);
        }

        if let Some(user_id) = &self.user_id {
            span.record("user_id", user_id);
        }

        if let Some(user_agent) = &self.user_agent {
            span.record("user_agent", user_agent);
        }

        if let Some(remote_ip) = &self.remote_ip {
            span.record("remote_ip", remote_ip);
        }

        if let Some(forwarded_for) = &self.forwarded_for {
            span.record("forwarded_for", forwarded_for);
        }
    }

    /// Add correlation headers to HTTP response
    pub fn add_to_response_headers(&self, headers: &mut HeaderMap) {
        if let Ok(header_name) = HeaderName::from_str(CORRELATION_ID_HEADER) {
            if let Ok(header_value) = HeaderValue::from_str(&self.correlation_id) {
                headers.insert(header_name, header_value);
            }
        }

        if let Ok(header_name) = HeaderName::from_str(REQUEST_ID_HEADER) {
            if let Ok(header_value) = HeaderValue::from_str(&self.request_id) {
                headers.insert(header_name, header_value);
            }
        }
    }
}

/// Middleware to extract and inject correlation context
pub async fn correlation_middleware(
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let correlation_ctx = CorrelationContext::from_headers(request.headers());

    // Store correlation context in request extensions for handlers to access
    request.extensions_mut().insert(correlation_ctx.clone());

    // Add correlation context to the current span
    let span = Span::current();
    correlation_ctx.add_to_span(&span);

    // Process the request
    let mut response = next.run(request).await;

    // Add correlation headers to response
    correlation_ctx.add_to_response_headers(response.headers_mut());

    response
}

fn extract_header_or_generate(headers: &HeaderMap, header_name: &str) -> String {
    extract_optional_header(headers, header_name).unwrap_or_else(|| Uuid::new_v4().to_string())
}

fn extract_optional_header(headers: &HeaderMap, header_name: &str) -> Option<String> {
    headers
        .get(header_name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}
