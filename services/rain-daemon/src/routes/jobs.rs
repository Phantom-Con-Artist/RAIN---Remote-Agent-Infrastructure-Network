use crate::response::{json_response, HttpResponse};
use crate::state::AppState;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::{Method, Request, StatusCode};
use rain_protocol::{ApiResponse, ArtifactTransferRequest, JobApprovalRequest};

pub(super) async fn handle(
    req: Request<Incoming>,
    state: &AppState,
) -> Result<HttpResponse, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    match (method, path.as_str()) {
        // --- JOBS & CROSS-MACHINE ORCHESTRATION ---
        (Method::GET, "/api/v1/jobs") => {
            let jobs = state.chat.list_jobs().await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(jobs)))
        }

        (Method::POST, path_str)
            if path_str.starts_with("/api/v1/jobs/") && path_str.ends_with("/approve") =>
        {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(job_id) = uuid::Uuid::parse_str(parts[4]) {
                    let body_bytes = req.into_body().collect().await?.to_bytes();
                    match serde_json::from_slice::<JobApprovalRequest>(&body_bytes) {
                        Ok(appr_req) => match state
                            .chat
                            .approve_job(job_id, appr_req.approved, appr_req.sudo_password)
                            .await
                        {
                            Ok(job) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(job))),
                            Err(e) => Ok(json_response(
                                StatusCode::NOT_FOUND,
                                &ApiResponse::<()>::err(e),
                            )),
                        },
                        Err(e) => Ok(json_response(
                            StatusCode::BAD_REQUEST,
                            &ApiResponse::<()>::err(format!("Invalid approval body: {}", e)),
                        )),
                    }
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid Job UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::GET, "/api/v1/mesh/resources") => {
            let res = state.remote.get_machine_resources().await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(res)))
        }

        (Method::POST, "/api/v1/mesh/transfer") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<ArtifactTransferRequest>(&body_bytes) {
                Ok(trans_req) => match state.remote.handle_artifact_transfer(trans_req) {
                    Ok(msg) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(msg))),
                    Err(e) => Ok(json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &ApiResponse::<()>::err(e),
                    )),
                },
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid transfer body: {}", e)),
                )),
            }
        }

        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err(format!("Route {} not found", path)),
        )),
    }
}
