use axum::http::HeaderMap;
use axum_test::TestRequest;

pub fn add_headers_to_request(mut request: TestRequest, headers: HeaderMap) -> TestRequest {
    for (name, value) in headers.iter() {
        request = request.add_header(name, value);
    }
    request
}
