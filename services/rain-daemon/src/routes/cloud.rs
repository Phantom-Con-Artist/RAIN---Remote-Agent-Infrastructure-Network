use crate::{
    response::{json_response, HttpResponse},
    state::AppState,
};
use http_body_util::BodyExt;
use hyper::{body::Incoming, Method, Request, StatusCode};
use rain_protocol::ApiResponse;
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;

fn reply(result: Result<Value, String>) -> HttpResponse {
    match result {
        Ok(v) => json_response(StatusCode::OK, &ApiResponse::ok(v)),
        Err(e) => json_response(StatusCode::BAD_REQUEST, &ApiResponse::<()>::err(e)),
    }
}
async fn body(req: Request<Incoming>) -> Result<Value, String> {
    let bytes = http_body_util::Limited::new(req.into_body(), 262144)
        .collect()
        .await
        .map_err(|_| "Request exceeds 256 KB")?
        .to_bytes();
    serde_json::from_slice(&bytes).map_err(|_| "Invalid JSON body".into())
}
pub(super) async fn handle(
    req: Request<Incoming>,
    state: &AppState,
) -> Result<HttpResponse, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let parts: Vec<_> = path.split('/').collect();
    if path.starts_with("/api/v1/cloud/agent/") {
        let token = req
            .headers()
            .get("x-rain-agent-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let Some(agent) = state.cloud.agent_auth(token).await else {
            return Ok(json_response(
                StatusCode::UNAUTHORIZED,
                &ApiResponse::<()>::err("Agent token invalid or revoked"),
            ));
        };
        if method == Method::GET && path == "/api/v1/cloud/agent/groups" {
            return Ok(reply(Ok(state.cloud.agent_groups(&agent).await)));
        }
        if method == Method::GET && path == "/api/v1/cloud/agent/guests" {
            return Ok(reply(Ok(state
                .cloud
                .guest_requests(Some(&agent), None)
                .await)));
        }
        if method == Method::POST && path == "/api/v1/cloud/agent/guests" {
            return Ok(reply(match body(req).await {
                Ok(v) => state.cloud.request_guest(&agent, v).await,
                Err(e) => Err(e),
            }));
        }
        if parts.len() == 8
            && parts[5] == "groups"
            && parts[7] == "messages"
            && method == Method::GET
        {
            let after = super::get_query_param(req.uri().query(), "after")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            return Ok(reply(
                state
                    .cloud
                    .group_records(parts[6], Some(&agent), after)
                    .await,
            ));
        }
        if parts.len() == 8
            && parts[5] == "groups"
            && parts[7] == "messages"
            && method == Method::POST
        {
            return Ok(reply(match body(req).await {
                Ok(v) => {
                    state
                        .cloud
                        .agent_message(&agent, parts[6], v["text"].as_str().unwrap_or("").into())
                        .await
                }
                Err(e) => Err(e),
            }));
        }
        if parts.len() == 9 && parts[5] == "groups" && parts[7] == "files" && method == Method::GET
        {
            if !state
                .cloud
                .group_file_allowed(&agent, parts[6], parts[8])
                .await
            {
                return Ok(json_response(
                    StatusCode::FORBIDDEN,
                    &ApiResponse::<()>::err("Group file access denied"),
                ));
            }
            let mut req = req;
            *req.uri_mut() = format!("/api/v1/artifacts/{}/content", parts[8])
                .parse()
                .unwrap();
            return super::artifacts::handle(req, state).await;
        }
        if parts.len() == 8 && parts[5] == "guests" && parts[7] == "tasks" && method == Method::POST
        {
            return Ok(reply(match body(req).await {
                Ok(v) => state.cloud.guest_task(&agent, parts[6], v).await,
                Err(e) => Err(e),
            }));
        }
        if parts.len() == 10
            && parts[5] == "groups"
            && parts[7] == "folders"
            && parts[9] == "tasks"
            && method == Method::POST
        {
            return Ok(reply(match body(req).await {
                Ok(v) => {
                    state
                        .cloud
                        .group_folder_task(&agent, parts[6], parts[8], v)
                        .await
                }
                Err(e) => Err(e),
            }));
        }
        if parts.len() == 7 && parts[5] == "tasks" && method == Method::GET {
            return Ok(reply(state.cloud.agent_task(&agent, parts[6]).await));
        }
        if parts.len() == 9 && parts[5] == "tasks" && parts[7] == "files" && method == Method::GET {
            match state.cloud.agent_task(&agent, parts[6]).await {
                Ok(t)
                    if t["outputs"]
                        .as_array()
                        .map(|ids| ids.iter().any(|id| id.as_str() == Some(parts[8])))
                        .unwrap_or(false) => {}
                _ => {
                    return Ok(json_response(
                        StatusCode::FORBIDDEN,
                        &ApiResponse::<()>::err("Guest file access denied"),
                    ))
                }
            }
            let mut req = req;
            *req.uri_mut() = format!("/api/v1/artifacts/{}/content", parts[8])
                .parse()
                .unwrap();
            return super::artifacts::handle(req, state).await;
        }
        return Ok(json_response(
            StatusCode::FORBIDDEN,
            &ApiResponse::<()>::err("Agent operation not permitted"),
        ));
    }
    if path.starts_with("/api/v1/cloud/owner/") {
        let worker = req
            .headers()
            .get("x-rain-owner-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let key = req
            .headers()
            .get("x-rain-owner-key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !state.cloud.owner_auth(&worker, key).await {
            return Ok(json_response(
                StatusCode::UNAUTHORIZED,
                &ApiResponse::<()>::err("Machine owner key required"),
            ));
        }
        if path == "/api/v1/cloud/owner/guests" && method == Method::GET {
            return Ok(reply(Ok(state
                .cloud
                .guest_requests(None, Some(&worker))
                .await)));
        }
        if parts.len() == 8
            && parts[5] == "guests"
            && parts[7] == "approve"
            && method == Method::POST
        {
            return Ok(reply(match body(req).await {
                Ok(v) => {
                    state
                        .cloud
                        .approve_guest(
                            parts[6],
                            Some(&worker),
                            v["approved"].as_bool() == Some(true),
                        )
                        .await
                }
                Err(e) => Err(e),
            }));
        }
        return Ok(json_response(
            StatusCode::FORBIDDEN,
            &ApiResponse::<()>::err("Machine owner operation not permitted"),
        ));
    }
    if path.starts_with("/api/v1/cloud/worker/") {
        if path == "/api/v1/cloud/worker/enroll" && method == Method::POST {
            return Ok(reply(match body(req).await {
                Ok(v) => {
                    state
                        .cloud
                        .enroll_credential(
                            v["invite"].as_str().unwrap_or(""),
                            v["worker_token"].as_str(),
                            v["owner_key_hash"].as_str(),
                        )
                        .await
                }
                Err(e) => Err(e),
            }));
        }
        let token = req
            .headers()
            .get("x-rain-worker-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let Some(worker) = state.cloud.authenticate(token).await else {
            return Ok(json_response(
                StatusCode::UNAUTHORIZED,
                &ApiResponse::<()>::err("Worker token invalid or revoked"),
            ));
        };
        if path == "/api/v1/cloud/worker/identity" && method == Method::GET {
            return Ok(reply(Ok(json!({"worker_id":worker}))));
        }
        if path == "/api/v1/cloud/worker/poll" && method == Method::POST {
            return Ok(reply(match body(req).await {
                Ok(v) => {
                    let resources = v.get("resources").cloned().unwrap_or(json!({}));
                    let programs: Vec<String> =
                        serde_json::from_value(v.get("programs").cloned().unwrap_or(json!([])))
                            .unwrap_or_default();
                    if programs.len() > 100 {
                        Err("Too many allowed programs".into())
                    } else {
                        state.cloud.poll(&worker, resources, programs).await
                    }
                }
                Err(e) => Err(e),
            }));
        }
        if parts.len() >= 8 && parts[5] == "tasks" {
            let task = parts[6];
            let action = parts[7];
            let lease = req
                .headers()
                .get("x-rain-task-lease")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            if action == "renew" && method == Method::POST {
                return Ok(reply(state.cloud.renew(&worker, task, &lease).await));
            }
            if action == "finish" && method == Method::POST {
                return Ok(reply(match body(req).await {
                    Ok(v) => state.cloud.finish(&worker, task, &lease, v).await,
                    Err(e) => Err(e),
                }));
            }
            let owned = match state.cloud.check_lease(&worker, task, &lease).await {
                Ok(t) => t,
                Err(e) => {
                    return Ok(json_response(
                        StatusCode::CONFLICT,
                        &ApiResponse::<()>::err(e),
                    ))
                }
            };
            if action == "inputs" && parts.len() == 9 && method == Method::GET {
                let id = parts[8];
                if !owned.inputs.iter().any(|i| i == id) {
                    return Ok(json_response(
                        StatusCode::FORBIDDEN,
                        &ApiResponse::<()>::err("This file is not an input of this task"),
                    ));
                }
                let mut request = req;
                *request.uri_mut() = format!("/api/v1/artifacts/{id}/content").parse().unwrap();
                return super::artifacts::handle(request, state).await;
            }
            if action == "outputs" && parts.len() == 8 && method == Method::POST {
                return Ok(reply(
                    upload(
                        req,
                        state,
                        format!("worker:{worker}"),
                        Some((&worker, task, &lease)),
                    )
                    .await,
                ));
            }
        }
        return Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err("Worker endpoint not found"),
        ));
    }
    match (method, path.as_str()) {
        (Method::GET, _) if parts.len() == 7 && parts[4] == "groups" && parts[6] == "messages" => {
            let after = super::get_query_param(req.uri().query(), "after")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            Ok(reply(
                state.cloud.group_records(parts[5], None, after).await,
            ))
        }
        (Method::GET, _) if parts.len() == 6 && parts[4] == "tasks" => {
            Ok(reply(state.cloud.task_record(parts[5]).await))
        }
        (Method::GET, "/api/v1/cloud/history") => {
            let offset = super::get_query_param(req.uri().query(), "offset")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            Ok(reply(Ok(state.cloud.history(offset).await)))
        }
        (Method::GET, "/api/v1/cloud/security") => {
            Ok(reply(Ok(state.cloud.security_snapshot().await)))
        }
        (Method::POST, "/api/v1/cloud/agents") => Ok(reply(match body(req).await {
            Ok(v) => state.cloud.managed_agent(v).await,
            Err(e) => Err(e),
        })),
        (Method::POST, "/api/v1/cloud/groups") => Ok(reply(match body(req).await {
            Ok(v) => state.cloud.group(v).await,
            Err(e) => Err(e),
        })),
        (Method::POST, _) if parts.len() == 7 && parts[4] == "groups" && parts[6] == "members" => {
            Ok(reply(match body(req).await {
                Ok(v) => match serde_json::from_value(v["members"].clone()) {
                    Ok(m) => state.cloud.group_members(parts[5], m).await,
                    Err(_) => Err("Choose group members".into()),
                },
                Err(e) => Err(e),
            }))
        }
        (Method::POST, _) if parts.len() == 7 && parts[4] == "groups" && parts[6] == "folders" => {
            Ok(reply(match body(req).await {
                Ok(v) => {
                    state
                        .cloud
                        .group_folder(
                            parts[5],
                            v["worker"].as_str().unwrap_or(""),
                            v["path"].as_str().unwrap_or(""),
                        )
                        .await
                }
                Err(e) => Err(e),
            }))
        }
        (Method::POST, _) if parts.len() == 8 && parts[4] == "groups" && parts[6] == "unshare" => {
            Ok(reply(
                state.cloud.remove_group_share(parts[5], parts[7]).await,
            ))
        }
        (Method::POST, _) if parts.len() == 7 && parts[4] == "agents" && parts[6] == "revoke" => {
            Ok(reply(state.cloud.revoke_agent(parts[5]).await))
        }
        (Method::POST, _) if parts.len() == 7 && parts[4] == "groups" && parts[6] == "files" => {
            Ok(reply(match body(req).await {
                Ok(v) => {
                    state
                        .cloud
                        .share_with_group(parts[5], v["file"].as_str().unwrap_or(""))
                        .await
                }
                Err(e) => Err(e),
            }))
        }
        (Method::POST, _) if parts.len() == 7 && parts[4] == "guests" && parts[6] == "approve" => {
            Ok(reply(match body(req).await {
                Ok(v) => {
                    state
                        .cloud
                        .approve_guest(parts[5], None, v["approved"].as_bool() == Some(true))
                        .await
                }
                Err(e) => Err(e),
            }))
        }
        (Method::GET, "/api/v1/cloud") => Ok(reply(Ok(state.cloud.snapshot().await))),
        (Method::POST, "/api/v1/cloud/invites") => Ok(reply(match body(req).await {
            Ok(v) => {
                state
                    .cloud
                    .invite(v["name"].as_str().unwrap_or("").into())
                    .await
            }
            Err(e) => Err(e),
        })),
        (Method::POST, "/api/v1/cloud/tasks") => Ok(reply(match body(req).await {
            Ok(v) => state.cloud.submit(v).await,
            Err(e) => Err(e),
        })),
        (Method::POST, "/api/v1/cloud/files") => {
            Ok(reply(upload(req, state, "hub".into(), None).await))
        }
        (Method::POST, _) if parts.len() == 7 && parts[4] == "workers" => {
            Ok(reply(state.cloud.set_worker(parts[5], parts[6]).await))
        }
        (Method::POST, _) if parts.len() == 7 && parts[4] == "tasks" => Ok(reply(match parts[6] {
            "cancel" => state.cloud.cancel(parts[5]).await,
            "retry" => state.cloud.retry(parts[5]).await,
            _ => Err("Unknown task action".into()),
        })),
        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err("Cloud endpoint not found"),
        )),
    }
}
async fn upload(
    req: Request<Incoming>,
    state: &AppState,
    source: String,
    lease: Option<(&str, &str, &str)>,
) -> Result<Value, String> {
    let name = super::get_query_param(req.uri().query(), "name").unwrap_or_else(|| "file".into());
    let name = std::path::Path::new(&name)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty() && *n != "." && *n != "..")
        .ok_or("Invalid filename")?
        .to_string();
    let folder = state.chat.artifacts.root().join("cloud-uploads");
    tokio::fs::create_dir_all(&folder)
        .await
        .map_err(|e| e.to_string())?;
    let path = folder.join(format!("{}-{name}", uuid::Uuid::new_v4()));
    let result = async {
        let mut options = tokio::fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&path).await.map_err(|e| e.to_string())?;
        let mut body = req.into_body();
        let mut total = 0usize;
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|e| e.to_string())?;
            if let Ok(bytes) = frame.into_data() {
                total += bytes.len();
                if total > 64 * 1024 * 1024 {
                    return Err("Shared file exceeds 64 MB".into());
                }
                file.write_all(&bytes).await.map_err(|e| e.to_string())?;
            }
        }
        file.sync_all().await.map_err(|e| e.to_string())?;
        drop(file);
        let artifact = state
            .chat
            .artifacts
            .capture(None, &path.to_string_lossy())
            .await?;
        let mut attachment =
            serde_json::to_value(artifact.attachment()).map_err(|e| e.to_string())?;
        attachment["name"] = json!(name);
        state.cloud.add_file(attachment, source, lease).await
    }
    .await;
    let _ = tokio::fs::remove_file(&path).await;
    result
}
