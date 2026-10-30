use crate::response::{json_response, HttpResponse};
use crate::state::AppState;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::{Method, Request, StatusCode};
use rain_protocol::{AgentLaunchRequest, ApiResponse, CapabilityInvocation, HealthStatus};

pub(super) async fn handle(
    req: Request<Incoming>,
    state: &AppState,
) -> Result<HttpResponse, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    match (method, path.as_str()) {
        (Method::GET, "/health") | (Method::GET, "/api/v1/health") => {
            let res = ApiResponse::ok(HealthStatus {
                status: "healthy".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                server_time: chrono::Utc::now(),
            });
            Ok(json_response(StatusCode::OK, &res))
        }

        // --- SYSTEM STATUS & CAPABILITIES ---
        (Method::GET, "/api/v1/system/status") => match state.registry.get_system_status().await {
            Ok(status) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(status))),
            Err(err) => Ok(json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ApiResponse::<()>::err(err),
            )),
        },

        (Method::GET, "/api/v1/system/capabilities") => {
            let caps = state.registry.list_capabilities();
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(caps)))
        }

        (Method::GET, "/api/v1/processes") => {
            let limit = 30;
            match state.registry.list_processes(limit).await {
                Ok(procs) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(procs))),
                Err(err) => Ok(json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ApiResponse::<()>::err(err),
                )),
            }
        }

        (Method::POST, "/api/v1/capabilities/invoke") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<CapabilityInvocation>(&body_bytes) {
                Ok(invocation) => {
                    let result = state.registry.invoke(invocation).await;
                    Ok(json_response(StatusCode::OK, &ApiResponse::ok(result)))
                }
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid request JSON: {}", e)),
                )),
            }
        }

        (Method::GET, "/api/v1/agents") => {
            let agents = state.supervisor.list_agents().await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(agents)))
        }

        (Method::GET, "/api/v1/agents/catalog") => {
            let machine = super::get_query_param(req.uri().query(), "machine");
            match rain_core::host_access::operate(
                machine.as_deref(),
                serde_json::json!({"action":"scan"}),
            )
            .await
            {
                Ok(data) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(data))),
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(e),
                )),
            }
        }
        (Method::GET, "/api/v1/agents/machines") => {
            let aliases: Vec<String> = vec![]; // Remote machines are enrolled in Cloud, never SSH aliases.
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(aliases)))
        }
        (Method::DELETE, path) if path.starts_with("/api/v1/agents/") => {
            let parts: Vec<_> = path.split('/').collect();
            let id = if parts.len() == 5 {
                uuid::Uuid::parse_str(parts[4]).ok()
            } else {
                None
            };
            match id {
                Some(id) => match state.chat.remove_session(id).await {
                    Ok(()) => Ok(json_response(
                        StatusCode::OK,
                        &ApiResponse::ok("Session removed; chat history retained"),
                    )),
                    Err(e) => Ok(json_response(
                        StatusCode::CONFLICT,
                        &ApiResponse::<()>::err(e),
                    )),
                },
                None => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err("Invalid session ID"),
                )),
            }
        }
        (Method::GET, path) if path.starts_with("/api/v1/agents/") && path.ends_with("/output") => {
            let id = path
                .split('/')
                .nth(4)
                .and_then(|id| uuid::Uuid::parse_str(id).ok());
            let after = super::get_query_param(req.uri().query(), "after")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            match id {
                Some(id) => match state.supervisor.get_agent(id).await {
                    Some(agent) => {
                        let available = agent.recent_logs.len() as u64;
                        let sequence = agent.output_sequence.max(available);
                        let reset = after > sequence || sequence.saturating_sub(after) > available;
                        let count = if reset {
                            available
                        } else {
                            sequence.saturating_sub(after)
                        } as usize;
                        let chunks = agent.recent_logs
                            [agent.recent_logs.len().saturating_sub(count)..]
                            .to_vec();
                        Ok(json_response(
                            StatusCode::OK,
                            &ApiResponse::ok(
                                serde_json::json!({"sequence":sequence,"chunks":chunks,"reset":reset,"interactive":agent.interactive,"status":agent.status}),
                            ),
                        ))
                    }
                    None => Ok(json_response(
                        StatusCode::NOT_FOUND,
                        &ApiResponse::<()>::err("Session not found"),
                    )),
                },
                None => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err("Invalid session ID"),
                )),
            }
        }
        (Method::POST, path)
            if path.starts_with("/api/v1/agents/") && path.ends_with("/resize") =>
        {
            let id = path
                .split('/')
                .nth(4)
                .and_then(|id| uuid::Uuid::parse_str(id).ok());
            let bytes = req.into_body().collect().await?.to_bytes();
            #[derive(serde::Deserialize)]
            struct Size {
                cols: u16,
                rows: u16,
            }
            match (id, serde_json::from_slice::<Size>(&bytes)) {
                (Some(id), Ok(size)) => match state
                    .supervisor
                    .resize_terminal(id, size.cols, size.rows)
                    .await
                {
                    Ok(()) => Ok(json_response(
                        StatusCode::OK,
                        &ApiResponse::ok("Terminal resized"),
                    )),
                    Err(e) => Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err(e),
                    )),
                },
                _ => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err("Invalid terminal size"),
                )),
            }
        }
        (Method::POST, path) if path.starts_with("/api/v1/agents/") && path.ends_with("/input") => {
            let id = path
                .split('/')
                .nth(4)
                .and_then(|id| uuid::Uuid::parse_str(id).ok());
            let body = req.into_body().collect().await?.to_bytes();
            match (
                id,
                serde_json::from_slice::<rain_protocol::AgentInputRequest>(&body),
            ) {
                (Some(id), Ok(input)) => match state.supervisor.send_raw(id, &input.input).await {
                    Ok(()) => Ok(json_response(
                        StatusCode::OK,
                        &ApiResponse::ok("Input sent"),
                    )),
                    Err(e) => Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err(e),
                    )),
                },
                _ => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err("Invalid session or input"),
                )),
            }
        }

        (Method::POST, "/api/v1/agents/start") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<AgentLaunchRequest>(&body_bytes) {
                Ok(launch_req) => match state.supervisor.start_agent(launch_req).await {
                    Ok(proc) => Ok(json_response(StatusCode::CREATED, &ApiResponse::ok(proc))),
                    Err(err) => Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err(err),
                    )),
                },
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid request body: {}", e)),
                )),
            }
        }

        (Method::POST, path_str)
            if path_str.starts_with("/api/v1/agents/") && path_str.ends_with("/stop") =>
        {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(id) = uuid::Uuid::parse_str(parts[4]) {
                    match state.supervisor.stop_agent(id).await {
                        Ok(_) => Ok(json_response(
                            StatusCode::OK,
                            &ApiResponse::ok("Agent terminated successfully"),
                        )),
                        Err(e) => Ok(json_response(
                            StatusCode::NOT_FOUND,
                            &ApiResponse::<()>::err(e),
                        )),
                    }
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid Agent UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err(format!("Route {} not found", path)),
        )),
    }
}
