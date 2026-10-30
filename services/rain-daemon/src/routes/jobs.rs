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
                Ok(trans_req) => match verified_transfer(state, trans_req).await {
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

        (Method::GET, "/api/v1/mesh/transfers") => Ok(json_response(
            StatusCode::OK,
            &ApiResponse::ok(state.chat.artifacts.transfers().await),
        )),
        (Method::POST, path)
            if path.starts_with("/api/v1/mesh/transfers/") && path.ends_with("/retry") =>
        {
            let id = path
                .split('/')
                .nth(5)
                .and_then(|v| uuid::Uuid::parse_str(v).ok());
            let original = state
                .chat
                .artifacts
                .transfers()
                .await
                .into_iter()
                .find(|t| Some(t.id) == id);
            let result = match original {
                Some(t) => {
                    state
                        .chat
                        .artifacts
                        .deliver(t.artifact_id, t.target_machine.as_deref())
                        .await
                }
                None => Err("Transfer record not found".into()),
            };
            Ok(match result {
                Ok(t) => json_response(StatusCode::OK, &ApiResponse::ok(t)),
                Err(e) => json_response(StatusCode::BAD_REQUEST, &ApiResponse::<()>::err(e)),
            })
        }
        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err(format!("Route {} not found", path)),
        )),
    }
}

async fn verified_transfer(
    state: &AppState,
    request: ArtifactTransferRequest,
) -> Result<rain_protocol::TransferReceipt, String> {
    let source = if request.source_machine.is_empty()
        || matches!(request.source_machine.as_str(), "local" | "host-primary")
    {
        None
    } else {
        Some(request.source_machine.as_str())
    };
    let target = if request.target_machine.is_empty()
        || matches!(request.target_machine.as_str(), "local" | "host-primary")
    {
        None
    } else {
        Some(request.target_machine.as_str())
    };
    let id = if let Some(id) = request.artifact_id {
        id
    } else if let Some(path) = request.source_path {
        state.chat.artifacts.capture(source, &path).await?.id
    } else if let Some(contents) = request.content_base64 {
        let file = rain_protocol::ChatAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            name: request.file_name,
            mime_type: "application/octet-stream".into(),
            size_bytes: request.size_bytes,
            sha256: request.sha256,
            data_base64: Some(contents),
            preview_content: None,
        };
        uuid::Uuid::parse_str(&state.chat.artifacts.ingest(&file).await?.id)
            .map_err(|e| e.to_string())?
    } else {
        return Err("Provide artifact_id, source_path, or actual file contents".into());
    };
    let mut receipt = state.chat.artifacts.deliver(id, target).await?;
    if !request.target_path.is_empty() {
        let copied=rain_core::host_access::operate_timeout(target,serde_json::json!({"action":"materialize","path":receipt.path,"destination":request.target_path,"sha256":receipt.sha256}),300).await?;
        receipt.path = copied["path"].as_str().map(str::to_owned);
    }
    Ok(receipt)
}
