use axum::{
    extract::Extension,
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware,
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_test::TestServer;
use backend::correlation::{
    correlation_middleware, CorrelationContext, CORRELATION_ID_HEADER, REQUEST_ID_HEADER,
    SESSION_ID_HEADER, USER_ID_HEADER,
};
use uuid::Uuid;

mod test_utils;
use test_utils::add_headers_to_request;

// Test handler that extracts correlation context and returns it
async fn test_handler(Extension(ctx): Extension<CorrelationContext>) -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "correlation_id": ctx.correlation_id,
        "request_id": ctx.request_id,
        "session_id": ctx.session_id,
        "user_id": ctx.user_id,
        "user_agent": ctx.user_agent,
        "remote_ip": ctx.remote_ip,
        "forwarded_for": ctx.forwarded_for
    }))
}

fn create_test_app() -> Router {
    Router::new()
        .route("/test", get(test_handler))
        .layer(middleware::from_fn(correlation_middleware))
}

#[tokio::test]
async fn test_correlation_middleware_with_existing_headers() {
    let app = create_test_app();
    let server = TestServer::new(app).expect("Failed to create test server");

    let correlation_id = "test-correlation-123";
    let request_id = "test-request-456";
    let session_id = "test-session-789";
    let user_id = "test-user-012";

    let mut headers = HeaderMap::new();
    headers.insert(
        CORRELATION_ID_HEADER,
        HeaderValue::from_static(correlation_id),
    );
    headers.insert(REQUEST_ID_HEADER, HeaderValue::from_static(request_id));
    headers.insert(SESSION_ID_HEADER, HeaderValue::from_static(session_id));
    headers.insert(USER_ID_HEADER, HeaderValue::from_static(user_id));
    headers.insert("user-agent", HeaderValue::from_static("test-browser/1.0"));
    headers.insert("cf-connecting-ip", HeaderValue::from_static("203.0.113.1"));
    headers.insert(
        "x-forwarded-for",
        HeaderValue::from_static("192.168.1.1, 10.0.0.1"),
    );

    let response = add_headers_to_request(server.get("/test"), headers).await;

    response.assert_status(StatusCode::OK);

    // Check response headers contain correlation info
    let response_headers = response.headers();
    assert_eq!(
        response_headers
            .get(CORRELATION_ID_HEADER)
            .unwrap()
            .to_str()
            .unwrap(),
        correlation_id
    );
    assert_eq!(
        response_headers
            .get(REQUEST_ID_HEADER)
            .unwrap()
            .to_str()
            .unwrap(),
        request_id
    );

    // Check response body contains expected values
    let body: serde_json::Value = response.json();
    assert_eq!(body["correlation_id"], correlation_id);
    assert_eq!(body["request_id"], request_id);
    assert_eq!(body["session_id"], session_id);
    assert_eq!(body["user_id"], user_id);
    assert_eq!(body["user_agent"], "test-browser/1.0");
    assert_eq!(body["remote_ip"], "203.0.113.1");
    assert_eq!(body["forwarded_for"], "192.168.1.1, 10.0.0.1");
}

#[tokio::test]
async fn test_correlation_middleware_generates_missing_ids() {
    let app = create_test_app();
    let server = TestServer::new(app).expect("Failed to create test server");

    // Don't provide correlation or request IDs
    let mut headers = HeaderMap::new();
    headers.insert("user-agent", HeaderValue::from_static("test-browser/2.0"));

    let response = add_headers_to_request(server.get("/test"), headers).await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();

    // Check that IDs were generated (should be valid UUIDs)
    let correlation_id = body["correlation_id"].as_str().unwrap();
    let request_id = body["request_id"].as_str().unwrap();

    assert!(
        Uuid::parse_str(correlation_id).is_ok(),
        "Invalid correlation ID UUID: {correlation_id}"
    );
    assert!(
        Uuid::parse_str(request_id).is_ok(),
        "Invalid request ID UUID: {request_id}"
    );

    // Optional fields should be null or have expected values
    assert_eq!(body["session_id"], serde_json::Value::Null);
    assert_eq!(body["user_id"], serde_json::Value::Null);
    assert_eq!(body["user_agent"], "test-browser/2.0");
    assert_eq!(body["remote_ip"], serde_json::Value::Null);
    assert_eq!(body["forwarded_for"], serde_json::Value::Null);

    // Check response headers contain generated IDs
    let response_headers = response.headers();
    assert_eq!(
        response_headers
            .get(CORRELATION_ID_HEADER)
            .unwrap()
            .to_str()
            .unwrap(),
        correlation_id
    );
    assert_eq!(
        response_headers
            .get(REQUEST_ID_HEADER)
            .unwrap()
            .to_str()
            .unwrap(),
        request_id
    );
}

#[tokio::test]
async fn test_ip_extraction_priority() {
    let app = create_test_app();
    let server = TestServer::new(app).expect("Failed to create test server");

    // Test Cloudflare IP takes priority
    let mut headers = HeaderMap::new();
    headers.insert("cf-connecting-ip", HeaderValue::from_static("203.0.113.10"));
    headers.insert("x-real-ip", HeaderValue::from_static("192.168.1.10"));
    headers.insert("x-forwarded-for", HeaderValue::from_static("10.0.0.10"));

    let response = add_headers_to_request(server.get("/test"), headers).await;
    let body: serde_json::Value = response.json();

    assert_eq!(body["remote_ip"], "203.0.113.10"); // Cloudflare should win
}

#[tokio::test]
async fn test_ip_extraction_fallback() {
    let app = create_test_app();
    let server = TestServer::new(app).expect("Failed to create test server");

    // Test fallback when no Cloudflare IP
    let mut headers = HeaderMap::new();
    headers.insert("x-real-ip", HeaderValue::from_static("192.168.1.20"));
    headers.insert("x-forwarded-for", HeaderValue::from_static("10.0.0.20"));

    let response = add_headers_to_request(server.get("/test"), headers).await;
    let body: serde_json::Value = response.json();

    assert_eq!(body["remote_ip"], "192.168.1.20"); // x-real-ip should win over x-forwarded-for
}

#[tokio::test]
async fn test_ip_extraction_x_forwarded_for_only() {
    let app = create_test_app();
    let server = TestServer::new(app).expect("Failed to create test server");

    // Test with only x-forwarded-for (should be in forwarded_for field, not remote_ip)
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", HeaderValue::from_static("10.0.0.30"));

    let response = add_headers_to_request(server.get("/test"), headers).await;
    let body: serde_json::Value = response.json();

    // x-forwarded-for goes to forwarded_for field, not remote_ip
    assert_eq!(body["remote_ip"], serde_json::Value::Null);
    assert_eq!(body["forwarded_for"], "10.0.0.30");
}

#[tokio::test]
async fn test_no_ip_headers() {
    let app = create_test_app();
    let server = TestServer::new(app).expect("Failed to create test server");

    // Test with no IP headers at all
    let response = server.get("/test").await;
    let body: serde_json::Value = response.json();

    assert_eq!(body["remote_ip"], serde_json::Value::Null);
    assert_eq!(body["forwarded_for"], serde_json::Value::Null);
}

#[tokio::test]
async fn test_correlation_context_creation() {
    let mut headers = HeaderMap::new();
    headers.insert(
        CORRELATION_ID_HEADER,
        HeaderValue::from_static("test-corr-id"),
    );
    headers.insert(REQUEST_ID_HEADER, HeaderValue::from_static("test-req-id"));
    headers.insert(SESSION_ID_HEADER, HeaderValue::from_static("test-session"));
    headers.insert(USER_ID_HEADER, HeaderValue::from_static("test-user"));
    headers.insert("user-agent", HeaderValue::from_static("test-agent"));
    headers.insert(
        "cf-connecting-ip",
        HeaderValue::from_static("203.0.113.100"),
    );
    headers.insert("x-forwarded-for", HeaderValue::from_static("192.168.1.100"));

    let ctx = CorrelationContext::from_headers(&headers);

    assert_eq!(ctx.correlation_id, "test-corr-id");
    assert_eq!(ctx.request_id, "test-req-id");
    assert_eq!(ctx.session_id, Some("test-session".to_string()));
    assert_eq!(ctx.user_id, Some("test-user".to_string()));
    assert_eq!(ctx.user_agent, Some("test-agent".to_string()));
    assert_eq!(ctx.remote_ip, Some("203.0.113.100".to_string()));
    assert_eq!(ctx.forwarded_for, Some("192.168.1.100".to_string()));
}

#[tokio::test]
async fn test_correlation_context_with_minimal_headers() {
    let headers = HeaderMap::new(); // Empty headers

    let ctx = CorrelationContext::from_headers(&headers);

    // Should generate UUIDs for required fields
    assert!(Uuid::parse_str(&ctx.correlation_id).is_ok());
    assert!(Uuid::parse_str(&ctx.request_id).is_ok());

    // Optional fields should be None
    assert_eq!(ctx.session_id, None);
    assert_eq!(ctx.user_id, None);
    assert_eq!(ctx.user_agent, None);
    assert_eq!(ctx.remote_ip, None);
    assert_eq!(ctx.forwarded_for, None);
}

#[tokio::test]
async fn test_correlation_headers_added_to_response() {
    let app = create_test_app();
    let server = TestServer::new(app).expect("Failed to create test server");

    let response = server.get("/test").await;

    response.assert_status(StatusCode::OK);

    let headers = response.headers();

    // Should have correlation headers in response
    assert!(headers.contains_key(CORRELATION_ID_HEADER));
    assert!(headers.contains_key(REQUEST_ID_HEADER));

    // Values should be valid UUIDs
    let correlation_id = headers
        .get(CORRELATION_ID_HEADER)
        .unwrap()
        .to_str()
        .unwrap();
    let request_id = headers.get(REQUEST_ID_HEADER).unwrap().to_str().unwrap();

    assert!(Uuid::parse_str(correlation_id).is_ok());
    assert!(Uuid::parse_str(request_id).is_ok());
}

#[tokio::test]
async fn test_multiple_requests_different_ids() {
    let app = create_test_app();
    let server = TestServer::new(app).expect("Failed to create test server");

    // Make first request
    let response1 = server.get("/test").await;
    let body1: serde_json::Value = response1.json();
    let correlation_id_1 = body1["correlation_id"].as_str().unwrap();
    let request_id_1 = body1["request_id"].as_str().unwrap();

    // Make second request
    let response2 = server.get("/test").await;
    let body2: serde_json::Value = response2.json();
    let correlation_id_2 = body2["correlation_id"].as_str().unwrap();
    let request_id_2 = body2["request_id"].as_str().unwrap();

    // IDs should be different for different requests
    assert_ne!(correlation_id_1, correlation_id_2);
    assert_ne!(request_id_1, request_id_2);
}
