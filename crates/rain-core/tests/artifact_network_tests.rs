use rain_core::{artifacts::ArtifactStore, host_access::operate, InMemoryStore};
use rain_protocol::{NetworkNode, PlatformKind, SpokeHeartbeatRequest, SpokeRegisterRequest};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn immutable_large_files_and_retired_ssh_transport() {
    let root = std::env::temp_dir().join(format!("rain-file-network-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let store = ArtifactStore::at(root.join("hub"));
    let bytes: Vec<u8> = (0..7 * 1024 * 1024 + 17).map(|i| (i % 251) as u8).collect();
    let source = root.join("shared.bin");
    std::fs::write(&source, &bytes).unwrap();
    let artifact = store.capture(None, source.to_str().unwrap()).await.unwrap();
    std::fs::write(&source, b"source changed after sharing").unwrap();
    assert_eq!(
        store.read(artifact.id, 123, 901).await.unwrap(),
        bytes[123..1024]
    );
    assert!(rain_core::host_access::ssh_target("source").is_err());
    assert!(store
        .capture(Some("source"), source.to_str().unwrap())
        .await
        .is_err());
    assert!(operate(Some("target"), json!({"action":"resources"}))
        .await
        .is_err());
    let restored = ArtifactStore::at(root.join("hub"));
    assert_eq!(
        restored.get(artifact.id).await.unwrap().sha256,
        artifact.sha256
    );
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
