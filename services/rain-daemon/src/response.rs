use bytes::Bytes;
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, Empty, Full};
use hyper::header::{
    HeaderValue, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE,
};
use hyper::{Response, StatusCode};

pub(crate) type HttpResponse =
    Response<UnsyncBoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>>>;

pub(crate) fn empty_body() -> UnsyncBoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed_unsync()
}

pub(crate) fn full_body<T: Into<Bytes>>(
    chunk: T,
) -> UnsyncBoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed_unsync()
}

pub(crate) fn json_response<T: serde::Serialize>(
    status: StatusCode,
    val: &T,
) -> Response<UnsyncBoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>>> {
    let json_bytes = serde_json::to_vec(val).unwrap_or_default();
    let mut res = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .header("cache-control", "no-store")
        .header("x-content-type-options", "nosniff")
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"))
        .header(
            ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, OPTIONS, PUT, PATCH, DELETE"),
        )
        .header(
            ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static(
                "Content-Type, Authorization, X-Rain-Device-Token, Range, If-Range",
            ),
        )
        .body(full_body(json_bytes))
        .unwrap();

    *res.status_mut() = status;
    res
}

pub(crate) fn cors_preflight(
) -> Response<UnsyncBoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>>> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"))
        .header(
            ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, OPTIONS, PUT, PATCH, DELETE"),
        )
        .header(
            ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static(
                "Content-Type, Authorization, X-Rain-Device-Token, Range, If-Range",
            ),
        )
        .body(empty_body())
        .unwrap()
}

pub(crate) fn html_response(
    status: StatusCode,
    html: String,
) -> Response<UnsyncBoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>>> {
    Response::builder()
        .status(status)
        .header("cache-control", "no-store")
        .header("x-content-type-options", "nosniff")
        .header("x-frame-options", "DENY")
        .header("referrer-policy", "no-referrer")
        .header("content-security-policy", "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' blob: data:; font-src 'self'; connect-src 'self' https: http://127.0.0.1:* http://localhost:*; frame-src 'self' blob:; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'")
        .header(
            CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"))
        .body(full_body(html.into_bytes()))
        .unwrap()
}

pub(crate) fn static_response(
    status: StatusCode,
    mime: &'static str,
    bytes: Vec<u8>,
) -> Response<UnsyncBoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, HeaderValue::from_static(mime))
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"))
        .body(full_body(bytes))
        .unwrap()
}
