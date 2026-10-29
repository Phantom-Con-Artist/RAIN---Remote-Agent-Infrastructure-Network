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
        r#"
  ██████╗  █████╗ ██╗███╗   ██╗
  ██╔══██╗██╔══██╗██║████╗  ██║
  ██████╔╝███████║██║██╔██╗ ██║
  ██╔══██╗██╔══██║██║██║╚██╗██║
  ██║  ██║██║  ██║██║██║ ╚████║
  ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝
  Remote Agent Infrastructure Network — Host Daemon v{}
  Messenger & Agent Orchestration Server
    "#,
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
        return run_spoke_agent(&hub_url, &spoke_name, pairing_code).await;
    }

    let store = Arc::new(InMemoryStore::new());
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
                    chat_events.record_event(event).await;
                    if let Err(e) = chat_events.persist(&event_path).await {
                        error!("History write failed: {}", e);
                    }
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
