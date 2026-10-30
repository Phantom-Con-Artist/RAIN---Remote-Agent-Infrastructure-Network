use crate::{
    response::{json_response, HttpResponse},
    state::AppState,
};
use hyper::{body::Incoming, Method, Request, StatusCode};
use rain_protocol::{ApiResponse, NetworkNode};
use serde_json::json;
use std::collections::HashMap;

pub(super) async fn handle(
    req: Request<Incoming>,
    state: &AppState,
) -> Result<HttpResponse, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/api/v1/network/topology") => {
            let mut system = sysinfo::System::new_all();
            system.refresh_memory();
            let root = NetworkNode {
                revoked: false,
                id: rain_core::network::local_node_id(),
                name: std::env::var("RAIN_NODE_NAME").unwrap_or_else(|_| {
                    sysinfo::System::host_name().unwrap_or_else(|| "RAIN hub".into())
                }),
                role: "hub".into(),
                parent_id: state.store.parent_path.read().await.last().cloned(),
                endpoint: Some(format!(
                    "http://127.0.0.1:{}",
                    std::env::var("RAIN_PORT").unwrap_or_else(|_| "7890".into())
                )),
                online: true,
                last_seen: chrono::Utc::now(),
                cpu_cores: system.cpus().len(),
                ram_total_bytes: system.total_memory(),
                ram_available_bytes: system.available_memory(),
                active_agents: state
                    .supervisor
                    .list_agents()
                    .await
                    .iter()
                    .filter(|a| a.status == rain_protocol::AgentStatus::Running)
                    .count(),
                children: state.store.network_children().await,
            };
            let hosts: HashMap<String, String> = serde_json::from_str(
                &std::env::var("RAIN_SSH_HOSTS").unwrap_or_else(|_| "{}".into()),
            )
            .unwrap_or_default();
            let health_path = state.chat.artifacts.root().join("host-health.json");
            let health: serde_json::Value = tokio::fs::read(health_path)
                .await
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or(json!({}));
            let transfers = state.chat.artifacts.transfers().await;
            let mut configured:Vec<_>=hosts.keys().map(|alias|{
    let last=health.get(alias).cloned().unwrap_or(json!({"status":"not_probed"}));
    let checked=last["checked_at"].as_str().and_then(|s|chrono::DateTime::parse_from_rfc3339(s).ok());
    let stale=checked.map(|t|(chrono::Utc::now()-t.with_timezone(&chrono::Utc)).num_seconds()>60).unwrap_or(false);
    json!({"alias":alias,"status":if stale{"stale"}else{last["status"].as_str().unwrap_or("not_probed")},"checked_at":last["checked_at"],"resources":last["resources"],"error":last["error"]})
   }).collect();
            configured.sort_by(|a, b| a["alias"].as_str().cmp(&b["alias"].as_str()));
            Ok(json_response(
                StatusCode::OK,
                &ApiResponse::ok(
                    json!({"model":"star_mesh_hybrid","control_plane":"hierarchical_hubs","data_plane":"peer_ssh_with_verified_hub_fallback","root":root,"parent_path":state.store.parent_path.read().await.clone(),"parent_link":{"url":std::env::var("RAIN_PARENT_HUB").ok(),"last_seen":*state.store.parent_last_seen.read().await,"online":state.store.parent_last_seen.read().await.map(|t|(chrono::Utc::now()-t).num_seconds()<60).unwrap_or(false)},"execution_hosts":configured,"transfers":transfers,"message_flows":state.chat.list_dispatches().await,"heartbeat_timeout_seconds":60}),
                ),
            ))
        }
        (&Method::POST, "/api/v1/network/probe") => {
            let hosts: HashMap<String, String> = serde_json::from_str(
                &std::env::var("RAIN_SSH_HOSTS").unwrap_or_else(|_| "{}".into()),
            )
            .unwrap_or_default();
            let mut tasks = tokio::task::JoinSet::new();
            for alias in hosts.into_keys() {
                tasks.spawn(async move{let value=match rain_core::host_access::operate(Some(&alias),json!({"action":"resources"})).await{Ok(resources)=>json!({"status":"online","checked_at":chrono::Utc::now(),"resources":resources}),Err(e)=>json!({"status":"unreachable","checked_at":chrono::Utc::now(),"error":e})};(alias,value)});
            }
            let mut result = serde_json::Map::new();
            while let Some(Ok((alias, status))) = tasks.join_next().await {
                result.insert(alias, status);
            }
            let root = state.chat.artifacts.root();
            let saved = async {
                tokio::fs::create_dir_all(root)
                    .await
                    .map_err(|e| e.to_string())?;
                let temporary = root.join(format!("host-health-{}.tmp", uuid::Uuid::new_v4()));
                tokio::fs::write(
                    &temporary,
                    serde_json::to_vec(&result).map_err(|e| e.to_string())?,
                )
                .await
                .map_err(|e| e.to_string())?;
                tokio::fs::rename(temporary, root.join("host-health.json"))
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            Ok(match saved {
                Ok(()) => json_response(StatusCode::OK, &ApiResponse::ok(result)),
                Err(e) => json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ApiResponse::<()>::err(e),
                ),
            })
        }
        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err("Network endpoint not found"),
        )),
    }
}
