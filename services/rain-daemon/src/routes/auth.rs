use crate::response::{json_response, HttpResponse};
use crate::state::AppState;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::{Method, Request, StatusCode};
use rain_protocol::{
    ApiResponse, PairingConfirmRequest, PairingConfirmResponse, PairingInitRequest,
    SessionStatusResponse,
};
use tracing::info;

pub(super) async fn handle(
    req: Request<Incoming>,
    state: &AppState,
) -> Result<HttpResponse, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    if path.starts_with("/api/v1/auth/otp/") {
        return Ok(json_response(
            StatusCode::NOT_IMPLEMENTED,
            &ApiResponse::<()>::err(
                "Email/SMS authentication is not configured. Use the infrastructure admin key.",
            ),
        ));
    }
    match (method, path.as_str()) {
        // --- AUTH & PAIRING ---
        (Method::POST, "/api/v1/auth/pair/init") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<PairingInitRequest>(&body_bytes) {
                Ok(init_req) => {
                    let res = state
                        .store
                        .create_pairing_code(
                            &init_req.client_device_id,
                            &init_req.client_device_name,
                            init_req.client_platform,
                        )
                        .await;
                    info!(
                        "Initiated legacy pairing for client '{}'",
                        init_req.client_device_name
                    );
                    Ok(json_response(StatusCode::OK, &ApiResponse::ok(res)))
                }
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid request: {}", e)),
                )),
            }
        }

        (Method::POST, "/api/v1/auth/pair/ticket") => {
            let host_name =
                std::env::var("HOSTNAME").unwrap_or_else(|_| "RAIN-Primary-Hub".to_string());
            let ticket = state
                .store
                .generate_pairing_ticket(&host_name, "0.0.0.0:7890")
                .await;
            info!("Generated legacy pairing ticket for Hub '{}'", host_name);
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(ticket)))
        }

        (Method::POST, "/api/v1/auth/pair/confirm") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<PairingConfirmRequest>(&body_bytes) {
                Ok(confirm_req) => {
                    match state
                        .store
                        .verify_pairing_code(
                            &confirm_req.client_device_id,
                            &confirm_req.pairing_code,
                        )
                        .await
                    {
                        Ok(token) => {
                            let resp = PairingConfirmResponse {
                                token,
                                approved: true,
                                host_name: std::env::var("HOSTNAME")
                                    .unwrap_or_else(|_| "RAIN-Host".to_string()),
                                device_id: confirm_req.client_device_id.clone(),
                                role: "collaborator".to_string(),
                            };
                            Ok(json_response(StatusCode::OK, &ApiResponse::ok(resp)))
                        }
                        Err(e) => Ok(json_response(
                            StatusCode::UNAUTHORIZED,
                            &ApiResponse::<()>::err(e),
                        )),
                    }
                }
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid request: {}", e)),
                )),
            }
        }

        (Method::GET, "/api/v1/auth/session") => {
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok());
            let token = req
                .headers()
                .get("x-rain-device-token")
                .and_then(|h| h.to_str().ok())
                .or_else(|| auth_header.and_then(|h| h.strip_prefix("Bearer ")));
            let host_name =
                std::env::var("HOSTNAME").unwrap_or_else(|_| "RAIN-Primary-Hub".to_string());

            if let Some(tok) = token {
                if let Some(device) = state.store.validate_token(tok).await {
                    let res = SessionStatusResponse {
                        authenticated: true,
                        device: Some(device),
                        hub_name: host_name.clone(),
                    };
                    return Ok(json_response(StatusCode::OK, &ApiResponse::ok(res)));
                }
            }

            // A reachable hub is not an authenticated device session.
            let res = SessionStatusResponse {
                authenticated: false,
                device: None,
                hub_name: host_name,
            };
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(res)))
        }

        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err(format!("Route {} not found", path)),
        )),
    }
}
