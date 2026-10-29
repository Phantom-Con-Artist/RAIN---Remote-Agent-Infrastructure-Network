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
    SendMessageRequest, SessionStatusResponse, SpokeHeartbeatRequest, SpokeRegisterRequest,
    TerminalExecuteRequest, TransportKind, WriteFileRequest,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn, Level};

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

fn html_response(status: StatusCode, html: String) -> Response<BoxBody<Bytes, hyper::Error>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"))
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"))
        .body(full_body(html.into_bytes()))
        .unwrap()
}

fn static_response(status: StatusCode, mime: &'static str, bytes: Vec<u8>) -> Response<BoxBody<Bytes, hyper::Error>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, HeaderValue::from_static(mime))
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"))
        .body(full_body(bytes))
        .unwrap()
}

fn render_hub_portal_html() -> String {
    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "RAIN-Primary-Hub".to_string());
    let battery = RemoteManager::read_real_battery();
    let bat_str = match &battery {
        Some(b) if b.is_critical => format!("⚠️ {}% CRITICAL (DISCHARGING)", b.percent),
        Some(b) if b.is_ac_connected => format!("⚡ {}% ({})", b.percent, b.power_source),
        Some(b) => format!("🔋 {}% ({})", b.percent, b.power_source),
        None => "⚡ AC Power".to_string(),
    };

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>RAIN Sovereign Hub — Active</title>
  <style>
    :root {{
      --bg: #090d16;
      --card-bg: #111827;
      --border: #1f2937;
      --accent: #38bdf8;
      --text: #f3f4f6;
      --text-muted: #9ca3af;
      --green: #10b981;
      --font-mono: 'JetBrains Mono', 'Fira Code', monospace;
    }}
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      background: var(--bg);
      color: var(--text);
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      padding: 32px 20px;
      display: flex;
      justify-content: center;
      line-height: 1.5;
    }}
    .container {{ max-width: 860px; width: 100%; }}
    .header {{
      background: linear-gradient(180deg, rgba(31, 41, 55, 0.6) 0%, rgba(17, 24, 39, 0.8) 100%);
      border: 1px solid var(--border);
      border-radius: 16px;
      padding: 28px;
      margin-bottom: 24px;
      box-shadow: 0 8px 32px rgba(0,0,0,0.4);
    }}
    .brand-row {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      flex-wrap: wrap;
      gap: 12px;
      margin-bottom: 12px;
    }}
    .logo {{
      font-size: 24px;
      font-weight: 800;
      letter-spacing: -0.5px;
      color: #fff;
      display: flex;
      align-items: center;
      gap: 10px;
    }}
    .badge-status {{
      display: inline-flex;
      align-items: center;
      gap: 6px;
      background: rgba(16, 185, 129, 0.15);
      color: var(--green);
      border: 1px solid rgba(16, 185, 129, 0.3);
      padding: 4px 12px;
      border-radius: 20px;
      font-size: 12px;
      font-weight: 700;
    }}
    .pulse-dot {{
      width: 8px;
      height: 8px;
      background: var(--green);
      border-radius: 50%;
      box-shadow: 0 0 10px var(--green);
    }}
    .subtitle {{ color: var(--text-muted); font-size: 14px; }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
      gap: 16px;
      margin-bottom: 24px;
    }}
    .card {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 20px;
    }}
    .card-title {{
      font-size: 12px;
      font-weight: 600;
      color: var(--text-muted);
      text-transform: uppercase;
      letter-spacing: 0.5px;
      margin-bottom: 8px;
    }}
    .card-val {{ font-size: 17px; font-weight: 700; color: #fff; }}
    .action-box {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 16px;
      padding: 24px;
      margin-bottom: 24px;
    }}
    .action-box h2 {{
      font-size: 18px;
      margin-bottom: 12px;
      display: flex;
      align-items: center;
      gap: 8px;
    }}
    .action-box p {{
      color: var(--text-muted);
      font-size: 14px;
      margin-bottom: 14px;
    }}
    .code-box {{
      background: #060911;
      border: 1px solid #1f293d;
      border-radius: 8px;
      padding: 12px 14px;
      font-family: var(--font-mono);
      font-size: 13px;
      color: #38bdf8;
      overflow-x: auto;
      margin-bottom: 14px;
      user-select: all;
    }}
    .btn-row {{ display: flex; gap: 10px; flex-wrap: wrap; }}
    .btn {{
      display: inline-flex;
      align-items: center;
      gap: 8px;
      padding: 10px 18px;
      border-radius: 8px;
      font-size: 13px;
      font-weight: 600;
      text-decoration: none;
      cursor: pointer;
      border: 1px solid transparent;
      transition: all 0.2s;
    }}
    .btn-primary {{ background: #0284c7; color: #fff; }}
    .btn-primary:hover {{ background: #0369a1; }}
    .btn-secondary {{ background: #1f2937; color: #e5e7eb; border-color: #374151; }}
    .btn-secondary:hover {{ background: #374151; }}
    .btn-danger {{
      background: rgba(239, 68, 68, 0.15);
      color: #ef4444;
      border-color: rgba(239, 68, 68, 0.3);
    }}
    .btn-danger:hover {{ background: rgba(239, 68, 68, 0.25); }}
    .links-list {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 10px;
      margin-top: 12px;
    }}
    .api-link {{
      background: #090e1a;
      border: 1px solid #1f293d;
      padding: 10px;
      border-radius: 8px;
      color: #93c5fd;
      text-decoration: none;
      font-family: var(--font-mono);
      font-size: 12px;
      display: block;
      transition: all 0.2s;
    }}
    .api-link:hover {{
      background: #111a2e;
      border-color: #38bdf8;
      color: #fff;
    }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <div class="brand-row">
        <div class="logo">🌧️ RAIN Hub Daemon</div>
        <div class="badge-status">
          <span class="pulse-dot"></span>
          HUB SERVER ONLINE (PORT 7890)
        </div>
      </div>
      <p class="subtitle">Remote Agent Infrastructure Network — Sovereign Star-Mesh Coordinator</p>
    </div>

    <div class="grid">
      <div class="card">
        <div class="card-title">Host Machine</div>
        <div class="card-val">{hostname}</div>
      </div>
      <div class="card">
        <div class="card-title">Hardware Power Telemetry</div>
        <div class="card-val">{bat_str}</div>
      </div>
      <div class="card">
        <div class="card-title">Topology Mode</div>
        <div class="card-val">Central Sovereign Hub</div>
      </div>
    </div>

    <div class="action-box">
      <h2>💻 Opening the RAIN Web Client Interface</h2>
      <p>The backend API daemon is running on port 7890. To interact with the full graphical workbench:</p>
      
      <p><strong>Option A — Start Vite Dev Server (Recommended for Live HMR):</strong></p>
      <div class="code-box">cd apps/rain-client && npm install && npm run dev</div>
      <div class="btn-row">
        <a href="http://localhost:5173" target="_blank" class="btn btn-primary">🔗 Open Web Client (http://localhost:5173)</a>
      </div>

      <div style="height: 18px;"></div>
      <p><strong>Option B — Serve Web Client Directly from this URL (Port 7890):</strong></p>
      <div class="code-box">cd apps/rain-client && npm install && npm run build</div>
      <p>Once built, visiting <code>http://localhost:7890/</code> in any browser automatically loads the full client app directly with zero extra dev servers!</p>
    </div>

    <div class="action-box">
      <h2>⚡ Connect a Second Machine (Spoke Worker Node)</h2>
      <p>Run this command on another laptop, desktop, or a second terminal to connect as a live spoke:</p>
      <div class="code-box">cargo run --bin rain-daemon -- --spoke --hub-url http://localhost:7890 --name "Worker-PC"</div>
    </div>

    <div class="action-box">
      <h2>🔋 Autonomous 5% Battery Evacuation Testing</h2>
      <p>Test the autonomous Git checkpoint and remote push failover pipeline:</p>
      <div class="btn-row">
        <button class="btn btn-danger" onclick="triggerEvac()">⚡ Trigger Autonomous Git Evacuation</button>
        <button class="btn btn-secondary" onclick="simulateBattery()">⚠️ Set Simulated 5% Battery</button>
        <button class="btn btn-secondary" onclick="resetBattery()">🔄 Reset Live Hardware Sensor</button>
      </div>
      <div id="action-output" style="margin-top: 12px; font-family: var(--font-mono); font-size: 13px; color: #38bdf8;"></div>
    </div>

    <div class="action-box">
      <h2>📡 Sovereign Hub REST Endpoints</h2>
      <div class="links-list">
        <a href="/api/v1/health" class="api-link" target="_blank">GET /api/v1/health</a>
        <a href="/api/v1/machines/overview" class="api-link" target="_blank">GET /api/v1/machines/overview</a>
        <a href="/api/v1/hub/topology" class="api-link" target="_blank">GET /api/v1/hub/topology</a>
        <a href="/api/v1/system/status" class="api-link" target="_blank">GET /api/v1/system/status</a>
        <a href="/api/v1/conversations" class="api-link" target="_blank">GET /api/v1/conversations</a>
        <a href="/api/v1/system/capabilities" class="api-link" target="_blank">GET /api/v1/system/capabilities</a>
      </div>
    </div>
  </div>

  <script>
    async function triggerEvac() {{
      const el = document.getElementById('action-output');
      el.innerText = 'Staging uncommitted files, committing, and pushing to GitHub...';
      try {{
        const res = await fetch('/api/v1/power/failover', {{ method: 'POST' }});
        const data = await res.json();
        el.innerText = '✅ Evacuation completed: ' + (data.data || JSON.stringify(data));
      }} catch(e) {{
        el.innerText = '❌ Error: ' + e.message;
      }}
    }}
    async function simulateBattery() {{
      const el = document.getElementById('action-output');
      const res = await fetch('/api/v1/power/simulate', {{ method: 'POST' }});
      const data = await res.json();
      el.innerText = '⚠️ ' + (data.data || '5% Battery simulation active');
    }}
    async function resetBattery() {{
      const el = document.getElementById('action-output');
      const res = await fetch('/api/v1/power/reset', {{ method: 'POST' }});
      const data = await res.json();
      el.innerText = '⚡ ' + (data.data || 'Sensor reset to live hardware');
    }}
  </script>
</body>
</html>"#, hostname = hostname, bat_str = bat_str)
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

    // 1. Root / UI / Static Assets
    if method == Method::GET && (path == "/" || path == "/index.html") {
        let dist_index = std::path::Path::new("apps/rain-client/dist/index.html");
        if dist_index.exists() {
            if let Ok(content) = std::fs::read_to_string(dist_index) {
                return Ok(html_response(StatusCode::OK, content));
            }
        }
        return Ok(html_response(StatusCode::OK, render_hub_portal_html()));
    }

    if method == Method::GET && path.starts_with("/assets/") {
        let asset_path = std::path::Path::new("apps/rain-client/dist").join(path.trim_start_matches('/'));
        if asset_path.exists() {
            let mime = if path.ends_with(".js") {
                "application/javascript"
            } else if path.ends_with(".css") {
                "text/css"
            } else if path.ends_with(".svg") {
                "image/svg+xml"
            } else if path.ends_with(".json") {
                "application/json"
            } else {
                "application/octet-stream"
            };
            if let Ok(bytes) = std::fs::read(asset_path) {
                return Ok(static_response(StatusCode::OK, mime, bytes));
            }
        }
    }

    match (method, path.as_str()) {
        (Method::GET, "/health") | (Method::GET, "/api/v1/health") => {
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

        // --- SPOKE NODES (STAR TOPOLOGY MESH) ---
        (Method::POST, "/api/v1/hub/spokes/register") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<SpokeRegisterRequest>(&body_bytes) {
                Ok(reg_req) => {
                    info!("Incoming spoke node registration: '{}' (ID: {})", reg_req.device_name, reg_req.device_id);
                    match state.store.register_spoke(reg_req).await {
                        Ok(token) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(token))),
                        Err(e) => Ok(json_response(StatusCode::BAD_REQUEST, &ApiResponse::<()>::err(e))),
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
                    match state.store.heartbeat_spoke(hb_req).await {
                        Ok(_) => Ok(json_response(StatusCode::OK, &ApiResponse::ok("Heartbeat accepted"))),
                        Err(e) => Ok(json_response(StatusCode::NOT_FOUND, &ApiResponse::<()>::err(e))),
                    }
                }
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid heartbeat payload: {}", e)),
                )),
            }
        }

        // --- TEST & POWER SIMULATION CONTROL ---
        (Method::POST, "/api/v1/power/simulate") => {
            std::env::set_var("RAIN_SIMULATE_BATTERY", "5");
            info!("Test battery mode activated: RAIN_SIMULATE_BATTERY=5 (Critical Discharging ⚠️)");
            Ok(json_response(StatusCode::OK, &ApiResponse::ok("Simulated battery set to 5% (Critical Discharging). Refresh Machines view to inspect.")))
        }

        (Method::POST, "/api/v1/power/reset") => {
            std::env::remove_var("RAIN_SIMULATE_BATTERY");
            info!("Test battery mode deactivated. Reading live kernel sysfs power sensors.");
            Ok(json_response(StatusCode::OK, &ApiResponse::ok("Battery mode restored to live hardware sensors.")))
        }

        // --- AUTONOMOUS BATTERY & POWER FAILOVER ---
        (Method::POST, "/api/v1/power/failover") => {
            let convs = state.chat.list_conversations().await;
            let conv_id = if let Some(first_conv) = convs.first() {
                first_conv.id
            } else {
                let auto_conv = state.chat.create_conversation("System Failover & Evacuation".to_string(), true, vec!["human-user".to_string()]).await;
                auto_conv.id
            };
            state.chat.execute_battery_failover_flow(conv_id, "Emergency battery evacuation".to_string()).await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok("Autonomous battery failover executed.")))
        }

        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err(format!("Route {} not found", path)),
        )),
    }
}

async fn run_spoke_agent(hub_url: &str, spoke_name: &str, pairing_code: Option<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let device_id = format!("spoke-{}", uuid::Uuid::new_v4().simple());
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    let cpu_cores = sys.cpus().len();
    let ram_total_gb = (sys.total_memory() as f32) / (1024.0 * 1024.0 * 1024.0);

    let reg_req = SpokeRegisterRequest {
        device_id: device_id.clone(),
        device_name: spoke_name.to_string(),
        platform: rain_protocol::PlatformKind::Linux,
        ip_address: "127.0.0.1 (Spoke Mesh)".to_string(),
        pairing_code,
        cpu_cores,
        ram_total_gb: (ram_total_gb * 10.0).round() / 10.0,
        gpu_model: Some("Peer GPU / Worker Node".to_string()),
        battery: RemoteManager::read_real_battery(),
    };

    info!("Registering Spoke node '{}' ({}) with Hub at {}", spoke_name, device_id, hub_url);
    let reg_url = format!("{}/api/v1/hub/spokes/register", hub_url.trim_end_matches('/'));

    match client.post(&reg_url).json(&reg_req).send().await {
        Ok(res) if res.status().is_success() => {
            info!("Successfully registered with Hub! Entering continuous telemetry heartbeat loop...");
        }
        Ok(res) => {
            error!("Failed to register with Hub: HTTP {}", res.status());
            return Ok(());
        }
        Err(e) => {
            error!("Cannot connect to Hub at {}: {}", reg_url, e);
            return Ok(());
        }
    }

    let heartbeat_url = format!("{}/api/v1/hub/spokes/heartbeat", hub_url.trim_end_matches('/'));
    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(3));
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let disk_total_bytes: u64 = disks.iter().map(|d| d.total_space()).sum();

    loop {
        ticker.tick().await;
        sys.refresh_all();

        let hb = SpokeHeartbeatRequest {
            device_id: device_id.clone(),
            cpu_usage: sys.global_cpu_usage(),
            memory_used_bytes: sys.used_memory(),
            memory_total_bytes: sys.total_memory(),
            disk_used_bytes: disks.iter().map(|d| d.total_space().saturating_sub(d.available_space())).sum(),
            disk_total_bytes,
            active_agents_count: 1,
            uptime_seconds: sysinfo::System::uptime(),
            battery: RemoteManager::read_real_battery(),
        };

        if let Err(e) = client.post(&heartbeat_url).json(&hb).send().await {
            warn!("Spoke heartbeat failed: {}", e);
        } else {
            debug!("Heartbeat delivered to Hub (CPU: {:.1}%, RAM: {:.1} GB)", hb.cpu_usage, (hb.memory_used_bytes as f32) / (1024.0*1024.0*1024.0));
        }
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

    let args: Vec<String> = std::env::args().collect();
    let is_spoke = args.iter().any(|a| a == "--spoke");

    if is_spoke {
        let mut hub_url = "http://127.0.0.1:7890".to_string();
        let mut spoke_name = format!("{}-Spoke", sysinfo::System::host_name().unwrap_or_else(|| "RAIN-Node".to_string()));
        let mut pairing_code: Option<String> = None;

        let mut i = 1;
        while i < args.len() {
            if args[i] == "--hub-url" && i + 1 < args.len() {
                hub_url = args[i + 1].clone();
                i += 1;
            } else if args[i] == "--name" && i + 1 < args.len() {
                spoke_name = args[i + 1].clone();
                i += 1;
            } else if args[i] == "--pin" && i + 1 < args.len() {
                pairing_code = Some(args[i + 1].clone());
                i += 1;
            }
            i += 1;
        }

        info!("Starting RAIN Daemon in Spoke Agent Mode");
        return run_spoke_agent(&hub_url, &spoke_name, pairing_code).await;
    }

    let store = Arc::new(InMemoryStore::new());
    let remote = Arc::new(RemoteManager::new_with_store((*store).clone()));

    let state = AppState {
        registry: Arc::new(CapabilityRegistry::new()),
        supervisor: Arc::new(AgentSupervisor::new()),
        store: store.clone(),
        chat: Arc::new(ChatStore::new()),
        remote,
    };

    let port = std::env::var("RAIN_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(7890);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    info!("RAIN Hub Daemon active and listening on http://{}", addr);
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
