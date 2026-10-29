use rain_core::{ChatStore, InMemoryStore, RemoteManager};
use rain_protocol::{PlatformKind, SpokeHeartbeatRequest, SpokeRegisterRequest};

#[tokio::test]
async fn test_real_battery_reader() {
    let battery = RemoteManager::read_real_battery();
    assert!(battery.is_some(), "Battery status should always resolve");
    let bat = battery.unwrap();
    println!("Live Hardware Battery: {}% ({}, Charging: {}, AC: {})", bat.percent, bat.power_source, bat.is_charging, bat.is_ac_connected);
    assert!(bat.percent <= 100, "Battery percentage should be <= 100");
}

#[tokio::test]
async fn test_battery_simulation_override() {
    std::env::set_var("RAIN_SIMULATE_BATTERY", "5");
    let battery = RemoteManager::read_real_battery().expect("Battery status expected");
    assert_eq!(battery.percent, 5);
    assert!(battery.is_critical, "5% must be critical");
    assert!(!battery.is_charging, "5% simulation must be discharging");
    std::env::remove_var("RAIN_SIMULATE_BATTERY");
}

#[tokio::test]
async fn test_spoke_registration_and_heartbeat() {
    let store = InMemoryStore::new();

    // 1. Register a live spoke
    let reg_req = SpokeRegisterRequest {
        device_id: "test-spoke-1".to_string(),
        device_name: "Ubuntu-Worker-Spoke".to_string(),
        platform: PlatformKind::Linux,
        ip_address: "192.168.1.188".to_string(),
        pairing_code: None,
        cpu_cores: 32,
        ram_total_gb: 64.0,
        gpu_model: Some("NVIDIA A100".to_string()),
        battery: None,
    };

    let token = store.register_spoke(reg_req).await.expect("Registration failed");
    assert!(token.starts_with("rain_spoke_"));

    // 2. Send heartbeat
    let hb_req = SpokeHeartbeatRequest {
        device_id: "test-spoke-1".to_string(),
        cpu_usage: 24.5,
        memory_used_bytes: 16 * 1024 * 1024 * 1024,
        memory_total_bytes: 64 * 1024 * 1024 * 1024,
        disk_used_bytes: 100 * 1024 * 1024 * 1024,
        disk_total_bytes: 500 * 1024 * 1024 * 1024,
        active_agents_count: 2,
        uptime_seconds: 3600,
        battery: None,
    };
    store.heartbeat_spoke(hb_req).await.expect("Heartbeat failed");

    // 3. Query active spokes
    let active = store.get_active_spokes().await;
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].device_name, "Ubuntu-Worker-Spoke");
    assert_eq!(active[0].cpu_usage, 24.5);

    // 4. Verify RemoteManager integrates active spokes into Machines overview
    let remote = RemoteManager::new_with_store(store.clone());
    let machines = remote.get_machines_overview().await;
    assert!(machines.iter().any(|m| m.id == "test-spoke-1" && m.name == "Ubuntu-Worker-Spoke"));

    // 5. Verify Hub Topology reports active spoke
    let topology = store.get_hub_topology().await;
    assert!(topology.spokes.iter().any(|s| s.device_id == "test-spoke-1" && s.is_online));
}

#[tokio::test]
async fn test_real_git_evacuation_flow() {
    let chat = ChatStore::new();
    let convs = chat.list_conversations().await;
    assert!(!convs.is_empty(), "Conversation list must have initial chat");
    let conv_id = convs[0].id;

    // Trigger battery evacuation
    chat.execute_battery_failover_flow(conv_id, "Emergency test".to_string()).await;

    // Verify messages created with real git artifacts
    let msgs = chat.get_messages(conv_id).await;
    let last_msg = msgs.last().expect("Messages should not be empty");
    assert!(last_msg.text.contains("Emergency failover received"));

    // Find the emergency alert message with the patch attachment
    let evac_msg = msgs.iter().find(|m| m.text.contains("CRITICAL POWER ALERT")).expect("Evac message missing");
    assert!(!evac_msg.attachments.is_empty(), "Evacuation message must have git patch attachment");
    let patch = &evac_msg.attachments[0];
    println!("Generated Real Git Patch: {} ({} bytes)", patch.name, patch.size_bytes);
    assert!(patch.name.starts_with("git-checkpoint-"));
    assert!(patch.name.ends_with(".patch"));
}

