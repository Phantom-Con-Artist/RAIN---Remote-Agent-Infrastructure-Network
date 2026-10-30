use rain_core::{artifacts::ArtifactStore, host_access::operate, InMemoryStore};
use rain_protocol::{NetworkNode, PlatformKind, SpokeHeartbeatRequest, SpokeRegisterRequest};
use serde_json::json;
use std::path::PathBuf;
use uuid::Uuid;

#[tokio::test]
async fn immutable_large_files_peer_copy_resume_fallback_and_corruption_recovery() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("rain-file-network-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let ssh = root.join("ssh-fixture");
    std::fs::write(&ssh,r#"#!/usr/bin/env python3
import sys,os,shlex
host=sys.argv[sys.argv.index('--')+1]
depth=int(os.environ.get('RAIN_FAKE_HOP','0'))
if depth and os.environ.get('RAIN_FAKE_DROP_PEER')=='1':
    print('Peer path intentionally unavailable',file=sys.stderr)
    sys.exit(255)
env=dict(os.environ,RAIN_FAKE_HOP=str(depth+1),RAIN_ARTIFACT_CACHE=os.path.join(os.environ['RAIN_FAKE_ROOT'],host))
args=shlex.split(sys.argv[-1])
os.execvpe(args[0],args,env)
"#).unwrap();
    std::fs::set_permissions(&ssh, std::fs::Permissions::from_mode(0o700)).unwrap();
    let variables = [
        "RAIN_SSH_BINARY",
        "RAIN_SSH_HOSTS",
        "RAIN_FAKE_ROOT",
        "RAIN_FAKE_DROP_PEER",
    ];
    let previous: Vec<_> = variables
        .iter()
        .map(|key| std::env::var(key).ok())
        .collect();
    std::env::set_var("RAIN_SSH_BINARY", &ssh);
    std::env::set_var(
        "RAIN_SSH_HOSTS",
        r#"{"source":"source","target":"target","relay":"relay"}"#,
    );
    std::env::set_var("RAIN_FAKE_ROOT", root.join("hosts"));
    let store = ArtifactStore::at(root.join("hub"));
    let bytes: Vec<u8> = (0..7 * 1024 * 1024 + 17).map(|i| (i % 251) as u8).collect();
    let source = root.join("outside-workspace-video.bin");
    std::fs::write(&source, &bytes).unwrap();
    let artifact = store
        .capture(Some("source"), source.to_str().unwrap())
        .await
        .unwrap();
    assert_eq!(artifact.size_bytes, bytes.len() as u64);
    std::fs::write(&source, b"changed after sharing").unwrap();
    assert_eq!(
        store.read(artifact.id, 123, 901).await.unwrap(),
        bytes[123..1024]
    );
    let restored = ArtifactStore::at(root.join("hub"));
    assert_eq!(
        restored.get(artifact.id).await.unwrap().sha256,
        artifact.sha256
    );
    let peer = store.deliver(artifact.id, Some("target")).await.unwrap();
    assert_eq!(peer.route, "peer_ssh");
    assert_eq!(std::fs::read(peer.path.unwrap()).unwrap(), bytes);
    // Prepare a partial receiver, then simulate a peer failure and a damaged disk prefix.
    std::env::set_var("RAIN_FAKE_DROP_PEER", "1");
    let stage=operate(Some("relay"),json!({"action":"stage","artifact_id":artifact.id,"name":artifact.name,"size_bytes":artifact.size_bytes,"sha256":artifact.sha256})).await.unwrap();
    let part = PathBuf::from(format!("{}.part", stage["path"].as_str().unwrap()));
    std::fs::write(&part, &bytes[..1024 * 1024]).unwrap();
    let resumed=operate(Some("relay"),json!({"action":"stage","artifact_id":artifact.id,"name":artifact.name,"size_bytes":artifact.size_bytes,"sha256":artifact.sha256})).await.unwrap();
    assert_eq!(resumed["offset"], 1024 * 1024);
    let mut damaged = bytes[..1024 * 1024].to_vec();
    damaged[0] ^= 255;
    std::fs::write(&part, damaged).unwrap();
    assert!(store.deliver(artifact.id, Some("relay")).await.is_err());
    let fallback = store.deliver(artifact.id, Some("relay")).await.unwrap();
    assert_eq!(fallback.route, "hub_relay");
    assert!(fallback.fallback_reason.is_some());
    assert_eq!(fallback.transferred_bytes, artifact.size_bytes);
    assert_eq!(std::fs::read(fallback.path.unwrap()).unwrap(), bytes);
    let records = restored.transfers().await;
    assert!(records.iter().any(|r| r.status == "failed"));
    assert!(records
        .iter()
        .any(|r| r.route == "peer_ssh" && r.status == "completed"));
    for (key, value) in variables.into_iter().zip(previous) {
        if let Some(value) = value {
            std::env::set_var(key, value);
        } else {
            std::env::remove_var(key);
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

fn node(id: &str, children: Vec<NetworkNode>) -> NetworkNode {
    NetworkNode {
        revoked: false,
        id: id.into(),
        name: id.into(),
        role: "hub".into(),
        parent_id: None,
        endpoint: None,
        online: true,
        last_seen: chrono::Utc::now(),
        cpu_cores: 4,
        ram_total_bytes: 16 * 1024 * 1024 * 1024,
        ram_available_bytes: 8 * 1024 * 1024 * 1024,
        active_agents: 0,
        children,
    }
}
#[tokio::test]
async fn hierarchy_persists_identity_expires_liveness_and_rejects_cycles() {
    let root = std::env::temp_dir().join(format!("rain-tree-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("network.json");
    let store = InMemoryStore::restore_network(&path).await.unwrap();
    let token = store
        .register_spoke(SpokeRegisterRequest {
            is_hub: true,
            endpoint: Some("http://subhub".into()),
            device_id: "subhub".into(),
            device_name: "Subhub".into(),
            platform: PlatformKind::Linux,
            ip_address: "local test".into(),
            pairing_code: None,
            cpu_cores: 4,
            ram_total_gb: 16.,
            gpu_model: None,
            battery: None,
        })
        .await
        .unwrap();
    let heartbeat = |children| SpokeHeartbeatRequest {
        children,
        device_id: "subhub".into(),
        cpu_usage: 10.,
        memory_used_bytes: 8 * 1024 * 1024 * 1024,
        memory_total_bytes: 16 * 1024 * 1024 * 1024,
        disk_used_bytes: 0,
        disk_total_bytes: 0,
        active_agents_count: 1,
        uptime_seconds: 10,
        battery: None,
    };
    store
        .heartbeat_spoke(heartbeat(vec![node(
            "subsubhub",
            vec![node("leaf", vec![])],
        )]))
        .await
        .unwrap();
    let tree = store.network_children().await;
    assert_eq!(tree[0].children[0].parent_id.as_deref(), Some("subhub"));
    assert_eq!(
        tree[0].children[0].children[0].parent_id.as_deref(),
        Some("subsubhub")
    );
    assert!(store
        .heartbeat_spoke(heartbeat(vec![node("subhub", vec![])]))
        .await
        .is_err());
    assert!(store
        .heartbeat_spoke(heartbeat(vec![node(
            &rain_core::network::local_node_id(),
            vec![]
        )]))
        .await
        .is_err());
    store.persist_network().await.unwrap();
    let restored = InMemoryStore::restore_network(&path).await.unwrap();
    assert!(restored.validate_token(&token).await.is_some());
    assert!(!restored.network_children().await[0].online);
    assert!(!restored.network_children().await[0].children[0].online);
    restored
        .heartbeat_spoke(heartbeat(vec![node("subsubhub", vec![])]))
        .await
        .unwrap();
    assert!(restored.network_children().await[0].online);
    assert!(restored.revoke_device("subhub").await);
    assert!(restored.validate_token(&token).await.is_none());
    assert!(restored.network_children().await[0].revoked);
    assert!(restored.get_active_spokes().await.is_empty());
    restored.persist_network().await.unwrap();
    assert!(InMemoryStore::restore_network(&path)
        .await
        .unwrap()
        .validate_token(&token)
        .await
        .is_none());
    std::fs::remove_dir_all(root).unwrap();
}
