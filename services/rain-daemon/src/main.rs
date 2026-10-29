use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::Incoming;
use hyper::header::{HeaderValue, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rain_core::{AgentSupervisor, CapabilityRegistry, ChatStore, InMemoryStore, RemoteManager};
use rain_protocol::{
    AgentLaunchRequest, ApiResponse, ArtifactTransferRequest, CapabilityInvocation,
    CreateConversationRequest, DirectChatRequest, EmailOtpRequest, EmailOtpVerifyRequest,
    HealthStatus, JobApprovalRequest, LaunchAppRequest, NetworkMeshStatus,
    PairingConfirmRequest, PairingConfirmResponse, PairingInitRequest,
    SendMessageRequest, SessionStatusResponse, TerminalExecuteRequest,
    TransportKind, WriteFileRequest,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, Level};

#[derive(Clone)]
struct AppState {
    registry: Arc<CapabilityRegistry>,
    supervisor: Arc<AgentSupervisor>,
    store: Arc<InMemoryStore>,
    chat: Arc<ChatStore>,
    remote: Arc<RemoteManager>,
}

fn empty_body() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn full_body<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

fn json_response<T: serde::Serialize>(
    status: StatusCode,
    val: &T,
) -> Response<BoxBody<Bytes, hyper::Error>> {
    let json_bytes = serde_json::to_vec(val).unwrap_or_default();
    let mut res = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"))
        .header(
            ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, OPTIONS, PUT, DELETE"),
        )
        .header(
            ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("Content-Type, Authorization"),
        )
        .body(full_body(json_bytes))
        .unwrap();

    *res.status_mut() = status;
    res
}

fn cors_preflight() -> Response<BoxBody<Bytes, hyper::Error>> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"))
        .header(
            ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, OPTIONS, PUT, DELETE"),
        )
        .header(
            ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("Content-Type, Authorization"),
        )
        .body(empty_body())
        .unwrap()
}

fn get_query_param(query: Option<&str>, key: &str) -> Option<String> {
    query?
        .split('&')
        .find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            if parts.next()? == key {
                let val = parts.next()?;
                let decoded = val.replace("%20", " ").replace("%2F", "/").replace("%7E", "~");
                Some(decoded)
            } else {
                None
            }
        })
}

async fn handle_request(
    req: Request<Incoming>,
    state: AppState,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| q.to_string());

    if method == Method::OPTIONS {
        return Ok(cors_preflight());
    }

    match (method, path.as_str()) {
        (Method::GET, "/health") => {
            let res = ApiResponse::ok(HealthStatus {
                status: "healthy".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                server_time: chrono::Utc::now(),
            });
            Ok(json_response(StatusCode::OK, &res))
        }

        // --- CONVERSATIONS & CHAT ---
        (Method::GET, "/api/v1/conversations") => {
            let convs = state.chat.list_conversations().await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(convs)))
        }

        (Method::POST, "/api/v1/conversations/create") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<CreateConversationRequest>(&body_bytes) {
                Ok(create_req) => {
                    let conv = state
                        .chat
                        .create_conversation(create_req.title, create_req.is_group, create_req.participant_ids)
                        .await;
                    Ok(json_response(StatusCode::CREATED, &ApiResponse::ok(conv)))
                }
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid create conversation JSON: {}", e)),
                )),
            }
        }

        (Method::POST, "/api/v1/conversations/direct") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<DirectChatRequest>(&body_bytes) {
                Ok(direct_req) => {
                    let conv = state.chat.get_or_create_direct_chat(&direct_req.agent_id).await;
                    Ok(json_response(StatusCode::OK, &ApiResponse::ok(conv)))
                }
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid direct chat JSON: {}", e)),
                )),
            }
        }

        (Method::DELETE, path_str) if path_str.starts_with("/api/v1/conversations/") => {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 5 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    match state.chat.delete_conversation(conv_id).await {
                        Ok(_) => Ok(json_response(
                            StatusCode::OK,
                            &ApiResponse::ok("Conversation deleted successfully"),
                        )),
                        Err(e) => Ok(json_response(
                            StatusCode::NOT_FOUND,
                            &ApiResponse::<()>::err(e),
                        )),
                    }
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::POST, path_str) if path_str.starts_with("/api/v1/conversations/") && path_str.ends_with("/stop") => {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    match state.chat.stop_conversation(conv_id).await {
                        Ok(conv) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(conv))),
                        Err(e) => Ok(json_response(
                            StatusCode::NOT_FOUND,
                            &ApiResponse::<()>::err(e),
                        )),
                    }
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::GET, path_str) if path_str.starts_with("/api/v1/conversations/") && path_str.ends_with("/messages") => {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    let msgs = state.chat.get_messages(conv_id).await;
                    Ok(json_response(StatusCode::OK, &ApiResponse::ok(msgs)))
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::POST, path_str) if path_str.starts_with("/api/v1/conversations/") && path_str.ends_with("/messages") => {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    let body_bytes = req.into_body().collect().await?.to_bytes();
                    match serde_json::from_slice::<SendMessageRequest>(&body_bytes) {
                        Ok(send_req) => {
                            match state
                                .chat
                                .send_human_message(
                                    conv_id,
                                    send_req.text,
                                    send_req.target_session_id,
                                    send_req.attachments,
                                )
                                .await
                            {
                                Ok(msg) => Ok(json_response(StatusCode::CREATED, &ApiResponse::ok(msg))),
                                Err(err) => Ok(json_response(
                                    StatusCode::BAD_REQUEST,
                                    &ApiResponse::<()>::err(err),
                                )),
                            }
                        }
                        Err(e) => Ok(json_response(
                            StatusCode::BAD_REQUEST,
                            &ApiResponse::<()>::err(format!("Invalid request body: {}", e)),
                        )),
                    }
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::GET, "/api/v1/sessions") => {
            let sessions = state.chat.list_sessions().await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(sessions)))
        }

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

        // --- JOBS & CROSS-MACHINE ORCHESTRATION ---
        (Method::GET, "/api/v1/jobs") => {
            let jobs = state.chat.list_jobs().await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(jobs)))
        }

        (Method::POST, path_str) if path_str.starts_with("/api/v1/jobs/") && path_str.ends_with("/approve") => {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(job_id) = uuid::Uuid::parse_str(parts[4]) {
                    let body_bytes = req.into_body().collect().await?.to_bytes();
                    match serde_json::from_slice::<JobApprovalRequest>(&body_bytes) {
                        Ok(appr_req) => match state.chat.approve_job(job_id, appr_req.approved, appr_req.sudo_password).await {
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

        (Method::GET, "/api/v1/fs/list") => {
            let req_path = get_query_param(query.as_deref(), "path").unwrap_or_else(|| "~".to_string());
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
                Ok(write_req) => match state.remote.write_file(&write_req.path, &write_req.content) {
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
                Ok(launch_req) => match state.remote.launch_app(&launch_req.app_id, launch_req.custom_args).await {
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

        // --- SYSTEM STATUS & CAPABILITIES ---
        (Method::GET, "/api/v1/system/status") => {
            match state.registry.get_system_status().await {
                Ok(status) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(status))),
                Err(err) => Ok(json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ApiResponse::<()>::err(err),
                )),
            }
        }

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

        (Method::POST, path_str) if path_str.starts_with("/api/v1/agents/") && path_str.ends_with("/stop") => {
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
                    info!("Initiated pairing for client '{}'. Code: {}", init_req.client_device_name, res.pairing_code);
                    Ok(json_response(StatusCode::OK, &ApiResponse::ok(res)))
                }
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid request: {}", e)),
                )),
            }
        }

        (Method::POST, "/api/v1/auth/pair/ticket") => {
            let host_name = std::env::var("HOSTNAME").unwrap_or_else(|_| "RAIN-Primary-Hub".to_string());
            let ticket = state.store.generate_pairing_ticket(&host_name, "0.0.0.0:7890").await;
            info!("Generated pairing ticket with PIN: {} for Hub '{}'", ticket.pairing_code, host_name);
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(ticket)))
        }

        (Method::POST, "/api/v1/auth/pair/confirm") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<PairingConfirmRequest>(&body_bytes) {
                Ok(confirm_req) => {
                    match state
                        .store
                        .verify_pairing_code(&confirm_req.client_device_id, &confirm_req.pairing_code)
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

        (Method::POST, "/api/v1/auth/otp/email/request") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<EmailOtpRequest>(&body_bytes) {
                Ok(otp_req) => {
                    let code = state.store.request_email_otp(otp_req).await;
                    info!("Email OTP generated for invitation: [CODE: {}]", code);
                    Ok(json_response(StatusCode::OK, &ApiResponse::ok(format!("Verification OTP sent. Test Code: {}", code))))
                }
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid request: {}", e)),
                )),
            }
        }

        (Method::POST, "/api/v1/auth/otp/email/verify") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<EmailOtpVerifyRequest>(&body_bytes) {
                Ok(verify_req) => {
                    match state.store.verify_email_otp(verify_req).await {
                        Ok(resp) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(resp))),
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
            let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
            let token = auth_header.and_then(|h| h.strip_prefix("Bearer "));
            let host_name = std::env::var("HOSTNAME").unwrap_or_else(|_| "RAIN-Primary-Hub".to_string());

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

            // Return default active session for seamless developer/USB testing
            let res = SessionStatusResponse {
                authenticated: true,
                device: None,
                hub_name: host_name,
            };
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(res)))
        }

        // --- STAR TOPOLOGY & SPOKES ---
        (Method::GET, "/api/v1/hub/topology") => {
            let topology = state.store.get_hub_topology().await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(topology)))
        }

        (Method::POST, path_str) if path_str.starts_with("/api/v1/hub/devices/") && path_str.ends_with("/revoke") => {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                let device_id = parts[4];
                let revoked = state.store.revoke_device(device_id).await;
                Ok(json_response(StatusCode::OK, &ApiResponse::ok(revoked)))
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        // --- AUTONOMOUS BATTERY & POWER FAILOVER ---
        (Method::POST, "/api/v1/power/failover") => {
            let convs = state.chat.list_conversations().await;
            if let Some(first_conv) = convs.first() {
                state.chat.execute_battery_failover_flow(first_conv.id, "Emergency battery evacuation".to_string()).await;
                Ok(json_response(StatusCode::OK, &ApiResponse::ok("Autonomous battery failover executed.")))
            } else {
                Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err("No active conversation found to dispatch failover notification"),
                ))
            }
        }

        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err(format!("Route {} not found", path)),
        )),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    println!(r#"
  ██████╗  █████╗ ██╗███╗   ██╗
  ██╔══██╗██╔══██╗██║████╗  ██║
  ██████╔╝███████║██║██╔██╗ ██║
  ██╔══██╗██╔══██║██║██║╚██╗██║
  ██║  ██║██║  ██║██║██║ ╚████║
  ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝
  Remote Agent Infrastructure Network — Host Daemon v{}
  Messenger & Agent Orchestration Server
    "#, env!("CARGO_PKG_VERSION"));

    let state = AppState {
        registry: Arc::new(CapabilityRegistry::new()),
        supervisor: Arc::new(AgentSupervisor::new()),
        store: Arc::new(InMemoryStore::new()),
        chat: Arc::new(ChatStore::new()),
        remote: Arc::new(RemoteManager::new()),
    };

    let port = std::env::var("RAIN_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(7890);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    info!("RAIN Daemon active and listening on http://{}", addr);
    info!("Registered {} capabilities", state.registry.list_capabilities().len());

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let state_clone = state.clone();

        tokio::spawn(async move {
            let service = service_fn(move |req| handle_request(req, state_clone.clone()));
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                error!("Connection error with {}: {:?}", remote_addr, err);
            }
        });
    }
}
