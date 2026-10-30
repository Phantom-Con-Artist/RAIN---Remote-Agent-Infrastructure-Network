use rain_protocol::NetworkNode;
use std::collections::HashSet;
pub fn local_node_id() -> String {
    std::env::var("RAIN_NODE_ID")
        .unwrap_or_else(|_| format!("hub:{}", sysinfo::System::host_name().unwrap_or_default()))
}
pub fn validate_subtree(nodes: &mut [NetworkNode], parent: &str) -> Result<(), String> {
    let mut seen = HashSet::from([local_node_id(), parent.into()]);
    fn visit(
        nodes: &mut [NetworkNode],
        parent: &str,
        seen: &mut HashSet<String>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 64 || seen.len() > 10000 {
            return Err(
                "Network hierarchy exceeds the supported metadata depth or node count".into(),
            );
        }
        for node in nodes {
            if node.id.is_empty() || !seen.insert(node.id.clone()) {
                return Err("Network hierarchy contains a cycle or duplicate node identity".into());
            }
            node.parent_id = Some(parent.into());
            visit(&mut node.children, &node.id, seen, depth + 1)?;
        }
        Ok(())
    }
    visit(nodes, parent, &mut seen, 0)
}
pub fn refresh_liveness(nodes: &mut [NetworkNode], parent_online: bool) {
    for node in nodes {
        node.online = !node.revoked
            && parent_online
            && node.online
            && (chrono::Utc::now() - node.last_seen).num_seconds() < 60;
        refresh_liveness(&mut node.children, node.online);
    }
}
