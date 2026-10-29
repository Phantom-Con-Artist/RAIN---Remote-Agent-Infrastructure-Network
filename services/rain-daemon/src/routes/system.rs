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
            let hosts: std::collections::HashMap<String, String> = serde_json::from_str(
                &std::env::var("RAIN_SSH_HOSTS").unwrap_or_else(|_| "{}".into()),
            )
            .unwrap_or_default();
            let mut aliases: Vec<_> = hosts.keys().cloned().collect();
            aliases.sort();
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(aliases)))
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
