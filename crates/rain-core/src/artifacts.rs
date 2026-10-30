//! Immutable file snapshots and verified, resumable star/mesh transfers. No file payloads in chat history.
use base64::{engine::general_purpose::STANDARD, Engine};
use rain_protocol::{ArtifactDescriptor, ChatAttachment, TransferReceipt};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;
const CHUNK: u64 = 1024 * 1024;

#[derive(Clone)]
pub struct ArtifactStore {
    root: Arc<PathBuf>,
}
impl Default for ArtifactStore {
    fn default() -> Self {
        Self::new()
    }
}
impl ArtifactStore {
    pub fn new() -> Self {
        let state = std::env::var("RAIN_STATE_PATH").unwrap_or_else(|_| ".rain/state.json".into());
        let root = std::env::var("RAIN_ARTIFACT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                Path::new(&state)
                    .parent()
                    .unwrap_or(Path::new(".rain"))
                    .join("artifacts")
            });
        Self::at(root)
    }
    pub fn at(root: PathBuf) -> Self {
        Self {
            root: Arc::new(if root.is_absolute() {
                root
            } else {
                std::env::current_dir().unwrap_or_default().join(root)
            }),
        }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub async fn get(&self, id: Uuid) -> Result<ArtifactDescriptor, String> {
        let bytes = tokio::fs::read(self.root.join("records").join(format!("{id}.json")))
            .await
            .map_err(|_| "Shared file is unavailable or was not registered".to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
    async fn save<T: serde::Serialize>(
        &self,
        category: &str,
        id: Uuid,
        value: &T,
    ) -> Result<(), String> {
        let folder = self.root.join(category);
        tokio::fs::create_dir_all(&folder)
            .await
            .map_err(|e| e.to_string())?;
        let temporary = folder.join(format!("{id}.{}.tmp", Uuid::new_v4()));
        let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        use tokio::io::AsyncWriteExt;
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&temporary).await.map_err(|e| e.to_string())?;
        file.write_all(&bytes).await.map_err(|e| e.to_string())?;
        file.sync_all().await.map_err(|e| e.to_string())?;
        drop(file);
        tokio::fs::rename(temporary, folder.join(format!("{id}.json")))
            .await
            .map_err(|e| e.to_string())
    }
    pub async fn capture(
        &self,
        machine: Option<&str>,
        path: &str,
    ) -> Result<ArtifactDescriptor, String> {
        let id = Uuid::new_v4();
        let mut request = json!({"action":"snapshot","artifact_id":id,"path":path});
        if machine.filter(|m| !m.is_empty()).is_none() {
            request["cache_root"] = json!(self.root.join("data"));
        }
        let receipt = crate::host_access::operate_timeout(machine, request, 300).await?;
        let artifact = ArtifactDescriptor {
            id,
            name: receipt["name"].as_str().ok_or("Missing filename")?.into(),
            mime_type: receipt["mime_type"]
                .as_str()
                .unwrap_or("application/octet-stream")
                .into(),
            size_bytes: receipt["size_bytes"].as_u64().ok_or("Missing file size")?,
            sha256: receipt["sha256"]
                .as_str()
                .ok_or("Missing file hash")?
                .into(),
            source_machine: machine.filter(|m| !m.is_empty()).map(str::to_owned),
            original_path: path.into(),
            snapshot_path: receipt["path"]
                .as_str()
                .ok_or("Missing snapshot path")?
                .into(),
            created_at: chrono::Utc::now(),
        };
        self.save("records", id, &artifact).await?;
        Ok(artifact)
    }
    pub async fn ingest(&self, file: &ChatAttachment) -> Result<ChatAttachment, String> {
        if let Some(encoded) = &file.data_base64 {
            let bytes = STANDARD
                .decode(encoded)
                .map_err(|_| "Invalid file encoding")?;
            if bytes.len() as u64 != file.size_bytes {
                return Err("Attachment size mismatch".into());
            }
            let folder = self.root.join("uploads").join(Uuid::new_v4().to_string());
            tokio::fs::create_dir_all(&folder)
                .await
                .map_err(|e| e.to_string())?;
            let name = Path::new(&file.name)
                .file_name()
                .and_then(|p| p.to_str())
                .filter(|s| !s.is_empty())
                .ok_or("Invalid filename")?;
            let path = folder.join(name);
            tokio::fs::write(&path, bytes)
                .await
                .map_err(|e| e.to_string())?;
            let artifact = self.capture(None, &path.to_string_lossy()).await?;
            if file
                .sha256
                .as_ref()
                .map(|h| h != &artifact.sha256)
                .unwrap_or(false)
            {
                return Err("Attachment SHA-256 mismatch".into());
            }
            let _ = tokio::fs::remove_file(path).await;
            Ok(artifact.attachment())
        } else {
            let id = Uuid::parse_str(&file.id).map_err(|_| "Invalid shared file ID")?;
            Ok(self.get(id).await?.attachment())
        }
    }
    pub async fn read(&self, id: Uuid, offset: u64, length: u64) -> Result<Vec<u8>, String> {
        let artifact = self.get(id).await?;
        let chunk = self
            .read_chunk(&artifact, offset, length.min(4 * CHUNK))
            .await?;
        STANDARD
            .decode(
                chunk["content_base64"]
                    .as_str()
                    .ok_or("Missing file chunk")?,
            )
            .map_err(|e| e.to_string())
    }
    async fn read_chunk(
        &self,
        a: &ArtifactDescriptor,
        offset: u64,
        length: u64,
    ) -> Result<Value, String> {
        retry(
            a.source_machine.as_deref(),
            json!({"action":"chunk","path":a.snapshot_path,"offset":offset,"length":length}),
        )
        .await
    }
    pub async fn deliver(&self, id: Uuid, target: Option<&str>) -> Result<TransferReceipt, String> {
        let a = self.get(id).await?;
        let target = target.filter(|s| !s.is_empty());
        let mut receipt = TransferReceipt {
            id: Uuid::new_v4(),
            artifact_id: id,
            source_machine: a.source_machine.clone(),
            target_machine: target.map(str::to_owned),
            route: "hub_relay".into(),
            status: "transferring".into(),
            bytes: a.size_bytes,
            transferred_bytes: 0,
            sha256: a.sha256.clone(),
            path: None,
            fallback_reason: None,
            error: None,
            started_at: chrono::Utc::now(),
            finished_at: None,
        };
        self.save("transfers", receipt.id, &receipt).await?;
        let result = self.copy(&a, target, &mut receipt).await;
        receipt.finished_at = Some(chrono::Utc::now());
        match result {
            Ok(path) => {
                receipt.status = "completed".into();
                receipt.transferred_bytes = a.size_bytes;
                receipt.path = Some(path);
                self.save("transfers", receipt.id, &receipt).await?;
                Ok(receipt)
            }
            Err(error) => {
                receipt.status = "failed".into();
                receipt.error = Some(error.clone());
                self.save("transfers", receipt.id, &receipt).await?;
                Err(error)
            }
        }
    }
    async fn copy(
        &self,
        a: &ArtifactDescriptor,
        target: Option<&str>,
        receipt: &mut TransferReceipt,
    ) -> Result<String, String> {
        if a.source_machine.as_deref() == target {
            receipt.route = "same_host".into();
            return Ok(a.snapshot_path.clone());
        }
        if a.source_machine.is_some() && target.is_some() {
            receipt.route = "peer_ssh".into();
            self.save("transfers", receipt.id, receipt).await?;
            let peer = json!({"action":"peer_copy","path":a.snapshot_path,"artifact_id":Uuid::new_v4(),"name":a.name,"size_bytes":a.size_bytes,"sha256":a.sha256,
                "ssh_target":crate::host_access::ssh_target(target.unwrap())?,"receiver_code":include_str!("host_probe.py"),"ssh_binary":std::env::var("RAIN_SSH_BINARY").unwrap_or_else(|_|"ssh".into())});
            match crate::host_access::operate_timeout(a.source_machine.as_deref(), peer, 300).await
            {
                Ok(done)
                    if done["sha256"].as_str() == Some(a.sha256.as_str())
                        && done["size_bytes"].as_u64() == Some(a.size_bytes) =>
                {
                    receipt.route = "peer_ssh".into();
                    return done["path"]
                        .as_str()
                        .map(str::to_owned)
                        .ok_or("Peer returned no path".into());
                }
                Ok(_) => receipt.fallback_reason = Some("Peer verification failed".into()),
                Err(e) => receipt.fallback_reason = Some(e),
            }
        }
        receipt.route = "hub_relay".into();
        self.save("transfers", receipt.id, receipt).await?;
        let mut request =
            json!({"artifact_id":a.id,"name":a.name,"size_bytes":a.size_bytes,"sha256":a.sha256});
        if target.is_none() {
            request["cache_root"] = json!(self.root.join("received"));
        }
        request["action"] = json!("stage");
        let stage = retry(target, request.clone()).await?;
        if stage["complete"].as_bool() == Some(true) {
            return stage["path"]
                .as_str()
                .map(str::to_owned)
                .ok_or("Missing staged path".into());
        }
        let mut offset = stage["offset"].as_u64().unwrap_or(0);
        while offset < a.size_bytes {
            let chunk = self.read_chunk(a, offset, CHUNK).await?;
            let count = chunk["size_bytes"]
                .as_u64()
                .filter(|n| *n > 0)
                .ok_or("Source ended before expected file size")?;
            request["action"] = json!("write_chunk");
            request["offset"] = json!(offset);
            request["content_base64"] = chunk["content_base64"].clone();
            request["chunk_sha256"] = chunk["sha256"].clone();
            let written = retry(target, request.clone()).await?;
            if written["offset"].as_u64() != Some(offset + count) {
                return Err("Destination acknowledged a different file range".into());
            }
            offset += count;
            receipt.transferred_bytes = offset;
            self.save("transfers", receipt.id, receipt).await?;
        }
        request["action"] = json!("finish_stage");
        request.as_object_mut().unwrap().remove("content_base64");
        let done = crate::host_access::operate_timeout(target, request, 300).await?;
        if done["sha256"].as_str() != Some(a.sha256.as_str()) {
            return Err("Destination hash mismatch".into());
        }
        done["path"]
            .as_str()
            .map(str::to_owned)
            .ok_or("Destination returned no path".into())
    }
    pub async fn transfers(&self) -> Vec<TransferReceipt> {
        let mut result = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(self.root.join("transfers")).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.path().extension().and_then(|x| x.to_str()) == Some("json") {
                    if let Ok(bytes) = tokio::fs::read(entry.path()).await {
                        if let Ok(record) = serde_json::from_slice::<TransferReceipt>(&bytes) {
                            result.push(record);
                        }
                    }
                }
            }
        }
        result.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        result.truncate(100);
        result
    }
}
async fn retry(machine: Option<&str>, request: Value) -> Result<Value, String> {
    let mut last = String::new();
    for attempt in 0..3 {
        match crate::host_access::operate(machine, request.clone()).await {
            Ok(v) => return Ok(v),
            Err(e) => last = e,
        }
        tokio::time::sleep(std::time::Duration::from_millis(100 * (1 << attempt))).await;
    }
    Err(last)
}
