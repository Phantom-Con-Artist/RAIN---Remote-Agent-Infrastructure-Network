use crate::{
    response::{json_response, HttpResponse},
    state::AppState,
};
use http_body_util::BodyExt;
use hyper::{body::Incoming, Method, Request, StatusCode};
use rain_protocol::ApiResponse;
use serde_json::{json, Value};
pub(super) async fn handle(
    req: Request<Incoming>,
    state: &AppState,
) -> Result<HttpResponse, hyper::Error> {
    let path = req.uri().path().to_string();
    if path == "/api/v1/admin/login" && req.method() == Method::POST {
        let secure = req
            .headers()
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            == Some("https");
        let bytes = match http_body_util::Limited::new(req.into_body(), 16384)
            .collect()
            .await
        {
            Ok(b) => b.to_bytes(),
            Err(_) => {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err("Login request too large"),
                ))
            }
        };
        let v: Value = serde_json::from_slice(&bytes).unwrap_or(json!({}));
        return Ok(
            match state.admin.login(v["key"].as_str().unwrap_or("")).await {
                Ok(session) => {
                    let mut response = json_response(
                        StatusCode::OK,
                        &ApiResponse::ok(
                            json!({"role":"sysadmin","expires_in":7200,"infrastructure":state.admin.identity()}),
                        ),
                    );
                    response.headers_mut().insert("set-cookie",format!("rain_admin={session}; HttpOnly; SameSite=Strict; Path=/; Max-Age=7200{}",if secure{"; Secure"}else{""}).parse().unwrap());
                    response
                        .headers_mut()
                        .insert("cache-control", "no-store".parse().unwrap());
                    response
                }
                Err(e) => json_response(StatusCode::UNAUTHORIZED, &ApiResponse::<()>::err(e)),
            },
        );
    }
    if !state.admin.authenticated(req.headers()).await {
        return Ok(json_response(
            StatusCode::UNAUTHORIZED,
            &ApiResponse::<()>::err("Admin key authentication required"),
        ));
    }
    match (req.method(), path.as_str()) {
        (&Method::GET, "/api/v1/admin/session") => Ok(json_response(
            StatusCode::OK,
            &ApiResponse::ok(json!({"role":"sysadmin","infrastructure":state.admin.identity()})),
        )),
        (&Method::GET, "/api/v1/admin/audit") => Ok(match state.admin.records().await {
            Ok(v) => json_response(StatusCode::OK, &ApiResponse::ok(v)),
            Err(e) => json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ApiResponse::<()>::err(e),
            ),
        }),
        (&Method::POST, "/api/v1/admin/logout") => {
            state.admin.logout(req.headers()).await;
            let mut response =
                json_response(StatusCode::OK, &ApiResponse::ok(json!({"logged_out":true})));
            response.headers_mut().insert(
                "set-cookie",
                "rain_admin=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"
                    .parse()
                    .unwrap(),
            );
            Ok(response)
        }
        (&Method::POST, "/api/v1/admin/rotate-key") => Ok(match state.admin.rotate().await {
            Ok(v) => json_response(StatusCode::OK, &ApiResponse::ok(v)),
            Err(e) => json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ApiResponse::<()>::err(e),
            ),
        }),
        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err("Admin endpoint not found"),
        )),
    }
}
