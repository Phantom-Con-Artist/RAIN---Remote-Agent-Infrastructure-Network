use crate::response::{json_response, HttpResponse};
use crate::state::AppState;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::{Method, Request, StatusCode};
use rain_protocol::{ApiResponse, SpokeHeartbeatRequest, SpokeRegisterRequest};
use tracing::info;

pub(super) async fn handle(
    req: Request<Incoming>,
    state: &AppState,
) -> Result<HttpResponse, hyper::Error> {
    let authorization = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_owned);
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    match (method, path.as_str()) {
        // --- STAR TOPOLOGY & SPOKES ---
        (Method::GET, "/api/v1/hub/topology") => {
            let topology = state.store.get_hub_topology().await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(topology)))
        }

        (Method::POST, path_str)
            if path_str.starts_with("/api/v1/hub/devices/") && path_str.ends_with("/revoke") =>
        {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 7 {
                let device_id = parts[5];
                let revoked = state.store.revoke_device(device_id).await;
                Ok(json_response(StatusCode::OK, &ApiResponse::ok(revoked)))
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        // --- SPOKE NODES (STAR TOPOLOGY MESH) ---
        (Method::POST, "/api/v1/hub/spokes/register") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<SpokeRegisterRequest>(&body_bytes) {
                Ok(reg_req) => {
                    if let Ok(key) = std::env::var("RAIN_CLUSTER_KEY") {
                        if authorization.as_deref() != Some(key.as_str())
                            && reg_req.pairing_code.is_none()
                        {
                            return Ok(json_response(StatusCode::UNAUTHORIZED,&ApiResponse::<()>::err("Node enrollment requires the cluster key or a valid pairing code")));
                        }
                    }
                    info!(
                        "Incoming spoke node registration: '{}' (ID: {})",
                        reg_req.device_name, reg_req.device_id
                    );
                    match state.store.register_spoke(reg_req).await {
                        Ok(token) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(token))),
                        Err(e) => Ok(json_response(
                            StatusCode::BAD_REQUEST,
                            &ApiResponse::<()>::err(e),
                        )),
                    }
                }
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid spoke registration payload: {}", e)),
                )),
            }
        }

        (Method::POST, "/api/v1/hub/spokes/heartbeat") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<SpokeHeartbeatRequest>(&body_bytes) {
                Ok(hb_req) => {
                    let device = match authorization.as_deref() {
                        Some(token) => state.store.validate_token(token).await,
                        None => None,
                    };
                    if !device
                        .as_ref()
                        .map(|d| d.device_id == hb_req.device_id)
                        .unwrap_or(false)
                    {
                        return Ok(json_response(
                            StatusCode::UNAUTHORIZED,
                            &ApiResponse::<()>::err("Heartbeat requires this node's token"),
                        ));
                    }
                    match state.store.heartbeat_spoke(hb_req).await {
                        Ok(_) => Ok(json_response(
                            StatusCode::OK,
                            &ApiResponse::ok("Heartbeat accepted"),
                        )),
                        Err(e) => Ok(json_response(
                            if e.starts_with("Spoke not found") {
                                StatusCode::NOT_FOUND
                            } else {
                                StatusCode::BAD_REQUEST
                            },
                            &ApiResponse::<()>::err(e),
                        )),
                    }
                }
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid heartbeat payload: {}", e)),
                )),
            }
        }

        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err(format!("Route {} not found", path)),
        )),
    }
}
