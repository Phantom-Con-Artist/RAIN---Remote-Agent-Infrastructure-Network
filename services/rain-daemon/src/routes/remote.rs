use super::get_query_param;
use crate::response::{json_response, HttpResponse};
use crate::state::AppState;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::{Method, Request, StatusCode};
use rain_protocol::{
    ApiResponse, LaunchAppRequest, NetworkMeshStatus, TerminalExecuteRequest, TransportKind,
    WriteFileRequest,
};

pub(super) async fn handle(
    req: Request<Incoming>,
    state: &AppState,
) -> Result<HttpResponse, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| q.to_string());

    match (method, path.as_str()) {
        // --- MACHINES & REMOTE WORKBENCH ---
        (Method::GET, "/api/v1/machines/overview") => {
            let machines = state.remote.get_machines_overview().await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(machines)))
        }

        (Method::GET, "/api/v1/network/status") => {
            let status = NetworkMeshStatus {
                current_transport: TransportKind::DirectUsb,
                node_id: "ed25519:7a4f91b8e3c0".to_string(),
                latency_ms: 7,
                bytes_sent: 142_900,
                bytes_received: 521_400,
                is_e2ee: true,
                available_transports: vec![
                    TransportKind::DirectUsb,
                    TransportKind::LanMdns,
                    TransportKind::WireguardP2p,
                    TransportKind::EncryptedRelay,
                ],
            };
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(status)))
        }

        (Method::GET, "/api/v1/fs/list") => {
            let req_path =
                get_query_param(query.as_deref(), "path").unwrap_or_else(|| "~".to_string());
            match state.remote.list_directory(&req_path) {
                Ok(entries) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(entries))),
                Err(e) => Ok(json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ApiResponse::<()>::err(e),
                )),
            }
        }

        (Method::GET, "/api/v1/fs/read") => {
            let req_path = get_query_param(query.as_deref(), "path").unwrap_or_default();
            match state.remote.read_file(&req_path) {
                Ok(res) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(res))),
                Err(e) => Ok(json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ApiResponse::<()>::err(e),
                )),
            }
        }

        (Method::POST, "/api/v1/fs/write") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<WriteFileRequest>(&body_bytes) {
                Ok(write_req) => match state.remote.write_file(&write_req.path, &write_req.content)
                {
                    Ok(_) => Ok(json_response(
                        StatusCode::OK,
                        &ApiResponse::ok("File saved successfully to machine"),
                    )),
                    Err(e) => Ok(json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &ApiResponse::<()>::err(e),
                    )),
                },
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid write request: {}", e)),
                )),
            }
        }

        (Method::GET, "/api/v1/apps/installed") => {
            let apps = state.remote.list_installed_apps();
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(apps)))
        }

        (Method::POST, "/api/v1/apps/launch") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<LaunchAppRequest>(&body_bytes) {
                Ok(launch_req) => match state
                    .remote
                    .launch_app(&launch_req.app_id, launch_req.custom_args)
                    .await
                {
                    Ok(msg) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(msg))),
                    Err(e) => Ok(json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &ApiResponse::<()>::err(e),
                    )),
                },
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid launch request: {}", e)),
                )),
            }
        }

        (Method::POST, "/api/v1/terminal/execute") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<TerminalExecuteRequest>(&body_bytes) {
                Ok(term_req) => match state.remote.execute_terminal(term_req).await {
                    Ok(term_res) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(term_res))),
                    Err(e) => Ok(json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &ApiResponse::<()>::err(e),
                    )),
                },
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid terminal request: {}", e)),
                )),
            }
        }

        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err(format!("Route {} not found", path)),
        )),
    }
}
