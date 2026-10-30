mod artifact_body;
mod portal;
mod response;
mod routes;
mod spoke;
mod state;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use rain_core::{AgentSupervisor, CapabilityRegistry, ChatStore, InMemoryStore, RemoteManager};
use routes::handle_request;
use spoke::run_spoke_agent;
use state::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    println!(
        "{}\n  Server v{}\n",
        rain_core::branding::BANNER,
        env!("CARGO_PKG_VERSION")
    );

    let args: Vec<String> = std::env::args().collect();
    let is_spoke = args.iter().any(|a| a == "--spoke");

    if is_spoke {
        let mut hub_url = "http://127.0.0.1:7890".to_string();
        let mut spoke_name = format!(
            "{}-Spoke",
            sysinfo::System::host_name().unwrap_or_else(|| "RAIN-Node".to_string())
        );
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
        return run_spoke_agent(&hub_url, &spoke_name, pairing_code, None, None).await;
    }

    let state_path = std::path::PathBuf::from(
        std::env::var("RAIN_STATE_PATH").unwrap_or_else(|_| ".rain/state.json".into()),
    );
    let runtime = state_path.parent().unwrap_or(std::path::Path::new(".rain"));
    tokio::fs::create_dir_all(runtime).await?;
    if std::env::var("RAIN_NODE_ID").is_err() {
        let path = runtime.join("node-id");
        let id = match tokio::fs::read_to_string(&path).await {
            Ok(id) => id.trim().to_string(),
            Err(_) => {
                let id = format!("hub-{}", uuid::Uuid::new_v4());
                tokio::fs::write(path, &id).await?;
                id
            }
        };
        std::env::set_var("RAIN_NODE_ID", id);
    }
    let store = Arc::new(
        InMemoryStore::restore_network(&runtime.join("network.json"))
            .await
            .map_err(std::io::Error::other)?,
    );
    let remote = Arc::new(RemoteManager::new_with_store((*store).clone()));

    let supervisor = Arc::new(AgentSupervisor::new());
    let history_path = std::path::PathBuf::from(
        std::env::var("RAIN_STATE_PATH").unwrap_or_else(|_| ".rain/state.json".into()),
    );
    let chat = Arc::new(
        ChatStore::restore((*supervisor).clone(), &history_path)
            .await
            .map_err(std::io::Error::other)?,
    );
    let event_path = history_path.clone();
    let mut events = supervisor.subscribe();
    let chat_events = chat.clone();
    tokio::spawn(async move {
        loop {
            match events.recv().await {
                Ok(event) => {
                    let chat = chat_events.clone();
                    let path = event_path.clone();
                    // Preserve native metadata order without serializing file work across channels.
                    if matches!(
                        &event,
                        rain_protocol::AgentEvent::TurnActivity { .. }
                            | rain_protocol::AgentEvent::Trace { .. }
                            | rain_protocol::AgentEvent::ActivityChanged { .. }
                    ) {
                        chat.record_event(event).await;
                        if let Err(e) = chat.persist(&path).await {
                            error!("History write failed: {}", e);
                        }
                        continue;
                    }
                    tokio::spawn(async move {
                        chat.record_event(event).await;
                        if let Err(e) = chat.persist(&path).await {
                            error!("History write failed: {}", e);
                        }
                    });
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Missed {} agent events", n)
                }
                Err(_) => break,
            }
        }
    });
    let state = AppState {
        history_path,
        registry: Arc::new(CapabilityRegistry::new()),
        supervisor,
        store: store.clone(),
        chat,
        remote,
    };

    let port = std::env::var("RAIN_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(7890);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;
    info!("RAIN Hub Daemon active and listening on http://{}", addr);
    info!(
        "Registered {} capabilities",
        state.registry.list_capabilities().len()
    );

    let parent = std::env::var("RAIN_PARENT_HUB").ok().or_else(|| {
        args.windows(2)
            .find(|w| w[0] == "--parent-hub")
            .map(|w| w[1].clone())
    });
    if let Some(parent) = parent {
        let store = state.store.clone();
        let supervisor = state.supervisor.clone();
        let name = std::env::var("RAIN_NODE_NAME").unwrap_or_else(|_| {
            sysinfo::System::host_name().unwrap_or_else(|| "RAIN subhub".into())
        });
        tokio::spawn(async move {
            if let Err(error) =
                run_spoke_agent(&parent, &name, None, Some(store), Some(supervisor)).await
            {
                error!("Parent connection stopped: {}", error);
            }
        });
    }
    let shutdown = async {
        #[cfg(unix)]
        {
            let mut terminate =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("SIGTERM handler");
            tokio::select! {_=terminate.recv()=>(),_=tokio::signal::ctrl_c()=>()}
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
    };
    tokio::pin!(shutdown);
    loop {
        let accepted =
            tokio::select! {connection=listener.accept()=>connection,_=&mut shutdown=>break};
        let (stream, remote_addr) = accepted?;
        let io = TokioIo::new(stream);
        let state_clone = state.clone();
        tokio::spawn(async move {
            let service = service_fn(move |req| handle_request(req, state_clone.clone()));
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                error!("Connection error with {}: {:?}", remote_addr, err);
            }
        });
    }
    info!("Stopping agent processes and saving state");
    let mut stopping = tokio::task::JoinSet::new();
    for agent in state.supervisor.list_agents().await {
        if agent.status == rain_protocol::AgentStatus::Running {
            let supervisor = state.supervisor.clone();
            stopping.spawn(async move {
                let _ = supervisor.stop_agent(agent.id).await;
            });
        }
    }
    while stopping.join_next().await.is_some() {}
    state
        .chat
        .persist(&state.history_path)
        .await
        .map_err(std::io::Error::other)?;
    state
        .store
        .persist_network()
        .await
        .map_err(std::io::Error::other)?;
    Ok(())
}
