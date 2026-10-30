//! Durable, single-owner task broker. Workers receive only their own leased work.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf};
use tokio::{io::AsyncWriteExt, sync::Mutex};

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
fn secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Worker {
    pub id: String,
    pub name: String,
    pub revoked: bool,
    pub last_seen: i64,
    pub resources: Value,
    pub enabled: bool,
    #[serde(default)]
    pub programs: Vec<String>,
    pub token: String,
    #[serde(default)]
    pub credential_hashed: bool,
    #[serde(default)]
    pub owner_hash: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Task {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub target: String,
    pub argv: Vec<String>,
    pub path: String,
    pub inputs: Vec<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub timeout_seconds: u64,
    pub request_id: String,
    pub worker_id: Option<String>,
    pub lease: Option<String>,
    pub lease_until: i64,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub agent_token: Option<String>,
    pub result: Value,
    pub outputs: Vec<String>,
    pub error: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Invite {
    token: String,
    name: String,
    expires_at: i64,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct SharedFile {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub created_at: i64,
    pub source: String,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Data {
    #[serde(default)]
    agents: BTreeMap<String, AgentIdentity>,
    #[serde(default)]
    groups: BTreeMap<String, Group>,
    #[serde(default)]
    guests: BTreeMap<String, Guest>,
    workers: BTreeMap<String, Worker>,
    tasks: BTreeMap<String, Task>,
    invites: Vec<Invite>,
    files: BTreeMap<String, SharedFile>,
}
pub(crate) struct Cloud {
    path: PathBuf,
    data: Mutex<Data>,
}
impl Cloud {
    pub async fn restore(path: PathBuf) -> Result<Self, String> {
        let mut data: Data = match tokio::fs::read(&path).await {
            Ok(b) => serde_json::from_slice(&b)
                .map_err(|e| format!("Cannot restore cloud state: {e}"))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Data::default(),
            Err(e) => return Err(e.to_string()),
        };
        for worker in data.workers.values_mut() {
            worker.last_seen = 0;
            if !worker.credential_hashed {
                worker.token = crate::admin::hash(&worker.token);
                worker.credential_hashed = true;
            }
        }
        // Running commands are never silently reassigned after a hub restart.
        for task in data.tasks.values_mut() {
            if task.status == "running" {
                task.status = "interrupted".into();
                task.error = Some(
                    "Hub restarted. Check effects before explicitly retrying this task.".into(),
                );
                task.updated_at = now();
            }
        }
        let cloud = Self {
            path,
            data: Mutex::new(data),
        };
        {
            let d = cloud.data.lock().await;
            cloud.save(&d).await?;
        }
        Ok(cloud)
    }
    async fn save(&self, data: &Data) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        let tmp = self
            .path
            .with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let mut opts = tokio::fs::OpenOptions::new();
        opts.create_new(true).write(true);
        #[cfg(unix)]
        opts.mode(0o600);
        let mut file = opts.open(&tmp).await.map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec(data).map_err(|e| e.to_string())?)
            .await
            .map_err(|e| e.to_string())?;
        file.sync_all().await.map_err(|e| e.to_string())?;
        drop(file);
        tokio::fs::rename(&tmp, &self.path)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    async fn change<T>(&self, f: impl FnOnce(&mut Data) -> Result<T, String>) -> Result<T, String> {
        let mut guard = self.data.lock().await;
        let mut next = guard.clone();
        let result = f(&mut next)?;
        self.save(&next).await?;
        *guard = next;
        Ok(result)
    }
    pub async fn expire(&self) -> Result<(), String> {
        let due = {
            let d = self.data.lock().await;
            d.tasks.values().any(|t| {
                (t.status == "running" && t.lease_until < now())
                    || (["running", "queued"].contains(&t.status.as_str()) && !guest_valid(&d, t))
            })
        };
        if due {
            self.change(|d| {
                let invalid: Vec<_> = d
                    .tasks
                    .values()
                    .filter(|t| !guest_valid(d, t))
                    .map(|t| t.id.clone())
                    .collect();
                for id in invalid {
                    if let Some(t) = d.tasks.get_mut(&id) {
                        if ["running", "queued"].contains(&t.status.as_str()) {
                            t.status = "cancelled".into();
                            t.updated_at = now();
                            t.error = Some("Guest permission expired or revoked".into());
                        }
                    }
                }
                for t in d.tasks.values_mut() {
                    if t.status == "running" && t.lease_until < now() {
                        t.status = "interrupted".into();
                        t.updated_at = now();
                        t.error =
                            Some("Worker lease expired. Inspect effects before retrying.".into());
                    }
                }
                Ok(())
            })
            .await?;
        }
        Ok(())
    }
    pub async fn snapshot(&self) -> Value {
        let d = self.data.lock().await;
        let workers:Vec<_>=d.workers.values().map(|w|json!({"id":w.id,"name":w.name,"revoked":w.revoked,"enabled":w.enabled,"online":!w.revoked&&now()-w.last_seen<30,"last_seen":w.last_seen,"resources":w.resources,"programs":w.programs})).collect();
        let mut recent: Vec<_> = d.tasks.values().collect();
        recent.sort_by_key(|t| (t.created_at, t.id.as_str()));
        let tasks: Vec<_> = recent
            .into_iter()
            .rev()
            .take(200)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|t| {
                let mut v = public_task(t);
                for key in ["stdout", "stderr"] {
                    if let Some(text) = v["result"][key].as_str() {
                        let preview = text.chars().take(4096).collect::<String>();
                        v["result"][key] = json!(preview);
                    }
                }
                v
            })
            .collect();
        json!({"hub":{"id":rain_core::network::local_node_id(),"name":std::env::var("RAIN_NODE_NAME").unwrap_or_else(|_|sysinfo::System::host_name().unwrap_or_else(||"My hub".into())),"uptime_seconds":sysinfo::System::uptime()},"workers":workers,"tasks":tasks,"task_count":d.tasks.len(),"files":d.files.values().collect::<Vec<_>>(),"server_time":now(),"model":"hub_owned_pull_workers","lease_seconds":30})
    }
    pub async fn invite(&self, name: String) -> Result<Value, String> {
        if name.trim().is_empty() || name.len() > 100 {
            return Err("Give the worker a name of at most 100 characters".into());
        }
        self.change(|d| {
            d.invites.retain(|i| i.expires_at > now());
            if d.invites.len() >= 20 {
                return Err("Too many active invitations".into());
            }
            let token = secret();
            let expires_at = now() + 300;
            d.invites.push(Invite {
                token: token.clone(),
                name,
                expires_at,
            });
            Ok(json!({"token":token,"expires_at":expires_at}))
        })
        .await
    }
    #[cfg(test)]
    pub async fn enroll(&self, token: &str) -> Result<Value, String> {
        self.enroll_credential(token, None, None).await
    }
    pub async fn enroll_credential(
        &self,
        invite: &str,
        credential: Option<&str>,
        owner: Option<&str>,
    ) -> Result<Value, String> {
        if credential
            .map(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
            .unwrap_or(false)
        {
            return Err("Invalid worker credential".into());
        }
        if owner
            .map(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
            .unwrap_or(false)
        {
            return Err("Invalid owner credential hash".into());
        }
        self.change(|d| {
            let index = d
                .invites
                .iter()
                .position(|i| i.token == invite && i.expires_at > now())
                .ok_or("Invitation expired, invalid or already used")?;
            let entry = d.invites.remove(index);
            let id = uuid::Uuid::new_v4().to_string();
            let token = credential.map(str::to_owned).unwrap_or_else(secret);
            d.workers.insert(
                id.clone(),
                Worker {
                    id: id.clone(),
                    name: entry.name,
                    revoked: false,
                    last_seen: 0,
                    resources: json!({}),
                    enabled: true,
                    programs: vec![],
                    token: crate::admin::hash(&token),
                    credential_hashed: true,
                    owner_hash: owner.unwrap_or("").into(),
                },
            );
            Ok(json!({"worker_id":id,"token":token}))
        })
        .await
    }
    pub async fn authenticate(&self, token: &str) -> Option<String> {
        if token.len() != 64 {
            return None;
        }
        self.data
            .lock()
            .await
            .workers
            .values()
            .find(|w| !w.revoked && constant_eq(&w.token, &crate::admin::hash(token)))
            .map(|w| w.id.clone())
    }
    pub async fn set_worker(&self, id: &str, action: &str) -> Result<Value, String> {
        self.change(|d| {
            let w = d.workers.get_mut(id).ok_or("Worker not found")?;
            match action {
                "revoke" => {
                    w.revoked = true;
                    w.enabled = false
                }
                "pause" => w.enabled = false,
                "resume" if !w.revoked => w.enabled = true,
                _ => return Err("Invalid worker action".into()),
            };
            if action == "revoke" {
                for a in d
                    .agents
                    .values_mut()
                    .filter(|a| a.worker.as_deref() == Some(id))
                {
                    a.revoked = true;
                }
                for g in d.guests.values_mut().filter(|g| g.worker == id) {
                    g.status = "revoked".into();
                }
                for t in d
                    .tasks
                    .values_mut()
                    .filter(|t| t.worker_id.as_deref() == Some(id) && t.status == "running")
                {
                    t.status = "cancelled".into();
                    t.updated_at = now();
                    t.error = Some("Worker revoked".into());
                    t.agent_token = None;
                }
            }
            Ok(json!({"updated":true}))
        })
        .await
    }
    pub async fn submit(&self, v: Value) -> Result<Value, String> {
        self.change(|d| {
            let request = v["request_id"]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 100)
                .ok_or("A unique request_id is required")?
                .to_string();
            if let Some(t) = d.tasks.values().find(|t| t.request_id == request) {
                return Ok(public_task(t));
            }
            if d.tasks.values().filter(|t| t.status == "queued").count() >= 1000 {
                return Err("Task queue is full".into());
            }
            let kind = v["kind"].as_str().unwrap_or("command").to_string();
            if !["command", "list_files", "fetch_file", "put_file"].contains(&kind.as_str()) {
                return Err("Unsupported task kind".into());
            }
            let argv: Vec<String> =
                serde_json::from_value(v.get("argv").cloned().unwrap_or(json!([])))
                    .map_err(|_| "argv must be an array of strings")?;
            if kind == "command"
                && (argv.is_empty()
                    || argv.len() > 128
                    || argv.iter().any(|s| s.len() > 8192 || s.contains('\0')))
            {
                return Err("Provide a command as an argument array, at most 128 arguments".into());
            }
            let title = v["title"]
                .as_str()
                .filter(|s| !s.trim().is_empty() && s.len() <= 200)
                .ok_or("Task title is required (at most 200 characters)")?
                .into();
            let target = v["target"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or("any")
                .to_string();
            if target != "any" && !d.workers.get(&target).map(|w| !w.revoked).unwrap_or(false) {
                return Err("Choose an approved worker".into());
            }
            let path = v["path"].as_str().unwrap_or("").to_string();
            if path.len() > 4096 || path.contains('\0') {
                return Err("Invalid shared path".into());
            }
            let inputs: Vec<String> =
                serde_json::from_value(v.get("inputs").cloned().unwrap_or(json!([])))
                    .map_err(|_| "inputs must be file IDs")?;
            if inputs.len() > 20 || inputs.iter().any(|id| !d.files.contains_key(id)) {
                return Err("Choose up to 20 files from the shared library".into());
            }
            if kind == "put_file" && inputs.len() != 1 {
                return Err("Choose exactly one shared file to send".into());
            }
            let id = uuid::Uuid::new_v4().to_string();
            let t = Task {
                id: id.clone(),
                title,
                kind,
                target,
                argv,
                path,
                inputs,
                status: "queued".into(),
                created_at: now(),
                updated_at: now(),
                timeout_seconds: v["timeout_seconds"].as_u64().unwrap_or(300).clamp(1, 3600),
                request_id: request,
                worker_id: None,
                lease: None,
                lease_until: 0,
                agent_id: None,
                agent_token: None,
                result: json!({}),
                outputs: vec![],
                error: None,
            };
            let result = public_task(&t);
            d.tasks.insert(id, t);
            Ok(result)
        })
        .await
    }
    pub async fn cancel(&self, id: &str) -> Result<Value, String> {
        self.change(|d| {
            let t = d.tasks.get_mut(id).ok_or("Task not found")?;
            if ["queued", "running"].contains(&t.status.as_str()) {
                t.status = "cancelled".into();
                t.updated_at = now();
            }
            Ok(public_task(t))
        })
        .await
    }
    pub async fn retry(&self, id: &str) -> Result<Value, String> {
        self.change(|d| {
            let mut t = d.tasks.get(id).ok_or("Task not found")?.clone();
            if t.agent_id
                .as_ref()
                .map(|id| d.agents.get(id).map(|a| a.revoked).unwrap_or(true))
                .unwrap_or(false)
            {
                return Err("Create a new credential before restarting a revoked agent".into());
            }
            if !["failed", "interrupted", "cancelled"].contains(&t.status.as_str()) {
                return Err("Only failed, interrupted or cancelled tasks can be retried".into());
            }
            t.id = uuid::Uuid::new_v4().to_string();
            t.request_id = secret();
            t.status = "queued".into();
            t.worker_id = None;
            t.lease = None;
            t.lease_until = 0;
            t.created_at = now();
            t.updated_at = now();
            t.outputs.clear();
            t.result = json!({});
            t.error = None;
            let result = public_task(&t);
            d.tasks.insert(t.id.clone(), t);
            Ok(result)
        })
        .await
    }
    pub async fn poll(
        &self,
        id: &str,
        resources: Value,
        programs: Vec<String>,
    ) -> Result<Value, String> {
        let mut guard = self.data.lock().await;
        let w = guard
            .workers
            .get_mut(id)
            .filter(|w| !w.revoked)
            .ok_or("Worker revoked")?;
        w.last_seen = now();
        w.resources = clean_resources(resources);
        w.programs = programs
            .into_iter()
            .filter(|s| !s.is_empty() && s.len() <= 512)
            .collect();
        if !w.enabled {
            return Ok(json!({"task":null}));
        }
        if guard
            .tasks
            .values()
            .any(|t| t.status == "running" && t.worker_id.as_deref() == Some(id))
        {
            return Ok(json!({"task":null}));
        }
        let candidate = guard
            .tasks
            .values()
            .filter(|t| {
                t.status == "queued"
                    && guest_valid(&guard, t)
                    && (t.target == "any" || t.target == id)
                    && (t.kind != "command"
                        || t.argv
                            .first()
                            .map(|p| guard.workers[id].programs.contains(p))
                            .unwrap_or(false))
            })
            .min_by_key(|t| t.created_at)
            .map(|t| t.id.clone());
        let Some(task_id) = candidate else {
            return Ok(json!({"task":null}));
        };
        let mut next = guard.clone();
        let t = next.tasks.get_mut(&task_id).unwrap();
        t.status = "running".into();
        t.worker_id = Some(id.into());
        t.lease = Some(secret());
        t.lease_until = now() + 30;
        t.updated_at = now();
        let result = json!({"task":t,"input_files":t.inputs.iter().filter_map(|id|next.files.get(id)).collect::<Vec<_>>()});
        self.save(&next).await?;
        *guard = next;
        Ok(result)
    }
    pub async fn check_lease(&self, id: &str, task_id: &str, lease: &str) -> Result<Task, String> {
        let d = self.data.lock().await;
        if !d.workers.get(id).map(|w| !w.revoked).unwrap_or(false) {
            return Err("Worker revoked".into());
        }
        let t = d.tasks.get(task_id).ok_or("Task not found")?;
        if !guest_valid(&d, t) {
            return Err("Guest grant expired or revoked".into());
        }
        if t.worker_id.as_deref() != Some(id)
            || t.lease.as_deref() != Some(lease)
            || t.lease_until < now()
            || t.status != "running"
        {
            return Err("Task lease expired or cancelled".into());
        }
        Ok(t.clone())
    }
    pub async fn renew(&self, id: &str, task_id: &str, lease: &str) -> Result<Value, String> {
        let mut d = self.data.lock().await;
        if !d.workers.get(id).map(|w| !w.revoked).unwrap_or(false) {
            return Err("Worker revoked".into());
        }
        let t = d.tasks.get(task_id).ok_or("Task not found")?;
        if !guest_valid(&d, t) {
            return Err("Guest grant expired or revoked".into());
        }
        if t.worker_id.as_deref() != Some(id)
            || t.lease.as_deref() != Some(lease)
            || t.status != "running"
            || t.lease_until < now()
        {
            return Err("Task lease expired or cancelled".into());
        }
        d.tasks.get_mut(task_id).unwrap().lease_until = now() + 30;
        d.workers.get_mut(id).unwrap().last_seen = now();
        Ok(json!({"active":true}))
    }
    pub async fn finish(
        &self,
        id: &str,
        task_id: &str,
        lease: &str,
        result: Value,
    ) -> Result<Value, String> {
        self.change(|d| {
            if !d.workers.get(id).map(|w| !w.revoked).unwrap_or(false) {
                return Err("Worker revoked".into());
            }
            if let Some(t) = d.tasks.get(task_id) {
                if !guest_valid(d, t) {
                    return Err("Guest grant expired or revoked".into());
                }
            }
            let t = d.tasks.get_mut(task_id).ok_or("Task not found")?;
            if t.worker_id.as_deref() != Some(id) || t.lease.as_deref() != Some(lease) {
                return Err("Task does not belong to this lease".into());
            }
            if ["completed", "failed"].contains(&t.status.as_str()) {
                return Ok(public_task(t));
            }
            if t.status != "running" || t.lease_until < now() {
                return Err("Task lease expired or cancelled".into());
            }
            let result = clean_result(result)?;
            t.status = if result["success"].as_bool() == Some(true) {
                "completed"
            } else {
                "failed"
            }
            .into();
            t.result = result;
            t.updated_at = now();
            Ok(public_task(t))
        })
        .await
    }
    pub async fn add_file(
        &self,
        attachment: Value,
        source: String,
        lease: Option<(&str, &str, &str)>,
    ) -> Result<Value, String> {
        self.change(|d| {
            let id = attachment["id"]
                .as_str()
                .ok_or("Missing file ID")?
                .to_string();
            if let Some((worker, task_id, key)) = lease {
                if !d.workers.get(worker).map(|w| !w.revoked).unwrap_or(false) {
                    return Err("Worker revoked".into());
                }
                let t = d.tasks.get_mut(task_id).ok_or("Task not found")?;
                if t.worker_id.as_deref() != Some(worker)
                    || t.lease.as_deref() != Some(key)
                    || t.status != "running"
                    || t.lease_until < now()
                {
                    return Err("Task lease expired or cancelled".into());
                }
                if t.outputs.len() >= 20 {
                    return Err("Task output file limit reached".into());
                }
                t.outputs.push(id.clone());
            }
            d.files.insert(
                id.clone(),
                SharedFile {
                    id,
                    name: attachment["name"].as_str().unwrap_or("file").into(),
                    size_bytes: attachment["size_bytes"].as_u64().unwrap_or(0),
                    sha256: attachment["sha256"].as_str().unwrap_or("").into(),
                    created_at: now(),
                    source,
                },
            );
            Ok(attachment)
        })
        .await
    }
}
fn public_task(t: &Task) -> Value {
    let mut v = serde_json::to_value(t).unwrap();
    v.as_object_mut().unwrap().remove("lease");
    v.as_object_mut().unwrap().remove("agent_token");
    v
}
fn constant_eq(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0, |n, (a, b)| n | (a ^ b)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn cloud() -> (Cloud, PathBuf) {
        let dir = std::env::temp_dir().join(format!("rain-cloud-test-{}", uuid::Uuid::new_v4()));
        let c = Cloud::restore(dir.join("cloud.json")).await.unwrap();
        (c, dir)
    }
    #[tokio::test]
    async fn one_time_enrollment_and_scoped_lease() {
        let (c, dir) = cloud().await;
        let invite = c.invite("Studio".into()).await.unwrap();
        let identity = c.enroll(invite["token"].as_str().unwrap()).await.unwrap();
        assert!(c.enroll(invite["token"].as_str().unwrap()).await.is_err());
        let worker = identity["worker_id"].as_str().unwrap();
        let t = c
            .submit(json!({"title":"test","request_id":"one","argv":["python3","-c","print(1)"]}))
            .await
            .unwrap();
        assert_eq!(
            c.submit(json!({"request_id":"one"})).await.unwrap()["id"],
            t["id"]
        );
        assert!(c.poll(worker, json!({}), vec![]).await.unwrap()["task"].is_null());
        let claimed = c
            .poll(worker, json!({}), vec!["python3".into()])
            .await
            .unwrap();
        let task = claimed["task"]["id"].as_str().unwrap();
        let lease = claimed["task"]["lease"].as_str().unwrap();
        assert!(c.renew(worker, task, "forged").await.is_err());
        assert!(c.snapshot().await["tasks"][0].get("lease").is_none());
        c.cancel(task).await.unwrap();
        assert!(c
            .finish(worker, task, lease, json!({"success":true}))
            .await
            .is_err());
        c.set_worker(worker, "revoke").await.unwrap();
        assert!(c
            .authenticate(identity["token"].as_str().unwrap())
            .await
            .is_none());
        tokio::fs::remove_dir_all(dir).await.unwrap();
    }
    #[tokio::test]
    async fn restart_interrupts_without_replay() {
        let (c, dir) = cloud().await;
        let invite = c.invite("worker".into()).await.unwrap();
        let id = c.enroll(invite["token"].as_str().unwrap()).await.unwrap();
        let worker = id["worker_id"].as_str().unwrap();
        c.submit(json!({"title":"work","kind":"list_files","request_id":"request"}))
            .await
            .unwrap();
        c.poll(worker, json!({}), vec![]).await.unwrap();
        let restored = Cloud::restore(dir.join("cloud.json")).await.unwrap();
        assert_eq!(
            restored.snapshot().await["tasks"][0]["status"],
            "interrupted"
        );
        assert!(restored.poll(worker, json!({}), vec![]).await.unwrap()["task"].is_null());
        tokio::fs::remove_dir_all(dir).await.unwrap();
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct AgentIdentity {
    id: String,
    name: String,
    token_hash: String,
    revoked: bool,
    #[serde(default)]
    worker: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Group {
    id: String,
    name: String,
    members: Vec<String>,
    files: Vec<String>,
    #[serde(default)]
    folders: Vec<Value>,
    messages: Vec<Value>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Guest {
    id: String,
    agent: String,
    worker: String,
    path: String,
    reason: String,
    status: String,
    expires_at: i64,
    approved_by: Option<String>,
    #[serde(default)]
    group: Option<String>,
}
impl Cloud {
    pub async fn security_snapshot(&self) -> Value {
        let d = self.data.lock().await;
        json!({"agents":d.agents.values().map(|a|json!({"id":a.id,"name":a.name,"revoked":a.revoked,"worker":a.worker,"task_id":a.task_id})).collect::<Vec<_>>(),"groups":d.groups.values().map(|g|json!({"id":g.id,"name":g.name,"members":g.members,"files":g.files,"folders":g.folders,"messages":g.messages.iter().rev().take(30).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>(),"message_count":g.messages.len()})).collect::<Vec<_>>(),"guests":d.guests.values().collect::<Vec<_>>()})
    }
    pub async fn create_agent(&self, name: String) -> Result<Value, String> {
        if name.trim().is_empty() || name.len() > 100 {
            return Err("Give the agent a name".into());
        }
        self.change(|d| {
            let id = uuid::Uuid::new_v4().to_string();
            let token = secret();
            d.agents.insert(
                id.clone(),
                AgentIdentity {
                    id: id.clone(),
                    name,
                    token_hash: crate::admin::hash(&token),
                    revoked: false,
                    worker: None,
                    task_id: None,
                },
            );
            Ok(json!({"agent_id":id,"token":token}))
        })
        .await
    }
    pub async fn agent_auth(&self, token: &str) -> Option<String> {
        if token.len() != 64 {
            return None;
        }
        let hash = crate::admin::hash(token);
        self.data
            .lock()
            .await
            .agents
            .values()
            .find(|a| !a.revoked && constant_eq(&a.token_hash, &hash))
            .map(|a| a.id.clone())
    }
    pub async fn revoke_agent(&self, id: &str) -> Result<Value, String> {
        self.change(|d| {
            d.agents.get_mut(id).ok_or("Agent not found")?.revoked = true;
            for t in d
                .tasks
                .values_mut()
                .filter(|t| t.agent_id.as_deref() == Some(id))
            {
                t.agent_token = None;
                if ["queued", "running"].contains(&t.status.as_str()) {
                    t.status = "cancelled".into();
                    t.updated_at = now();
                }
            }
            for grant in d.guests.values_mut().filter(|g| g.agent == id) {
                grant.status = "revoked".into();
            }
            Ok(json!({"revoked":true}))
        })
        .await
    }
    pub async fn group(&self, v: Value) -> Result<Value, String> {
        self.change(|d| {
            let name = v["name"]
                .as_str()
                .filter(|s| !s.trim().is_empty() && s.len() <= 100)
                .ok_or("Group name required")?
                .to_string();
            let members: Vec<String> =
                serde_json::from_value(v["members"].clone()).map_err(|_| "Choose group members")?;
            if members.len() > 100
                || members
                    .iter()
                    .any(|id| !d.agents.get(id).map(|a| !a.revoked).unwrap_or(false))
            {
                return Err("Group members must be approved agents".into());
            }
            let id = uuid::Uuid::new_v4().to_string();
            let group = Group {
                id: id.clone(),
                name,
                members,
                files: vec![],
                folders: vec![],
                messages: vec![],
            };
            let result = serde_json::to_value(&group).unwrap();
            d.groups.insert(id, group);
            Ok(result)
        })
        .await
    }
    pub async fn share_with_group(&self, id: &str, file: &str) -> Result<Value, String> {
        self.change(|d| {
            if !d.files.contains_key(file) {
                return Err("Shared file not found".into());
            }
            let group = d.groups.get_mut(id).ok_or("Group not found")?;
            if !group.files.contains(&file.to_string()) {
                group.files.push(file.into());
            }
            Ok(json!({"shared_read_only":true}))
        })
        .await
    }
    pub async fn agent_groups(&self, agent: &str) -> Value {
        let d = self.data.lock().await;
        json!(d.groups.values().filter(|g|g.members.iter().any(|id|id==agent)).map(|g|json!({"id":g.id,"name":g.name,"members":g.members,"messages":g.messages.iter().rev().take(100).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>(),"message_count":g.messages.len(),"folders":g.folders,"files":g.files.iter().filter_map(|id|d.files.get(id)).collect::<Vec<_>>()})).collect::<Vec<_>>())
    }
    pub async fn agent_message(
        &self,
        agent: &str,
        group: &str,
        text: String,
    ) -> Result<Value, String> {
        if text.trim().is_empty() || text.len() > 16384 {
            return Err("Message must contain 1–16384 bytes".into());
        }
        self.change(|d|{let group=d.groups.get_mut(group).filter(|g|g.members.iter().any(|id|id==agent)).ok_or("Group access denied")?;if group.messages.len()>=10000{return Err("Group message limit reached".into());}let message=json!({"id":uuid::Uuid::new_v4().to_string(),"sender":agent,"text":text,"created_at":now(),"sequence":group.messages.len()});group.messages.push(message.clone());Ok(message)}).await
    }
    pub async fn group_file_allowed(&self, agent: &str, group: &str, file: &str) -> bool {
        self.data
            .lock()
            .await
            .groups
            .get(group)
            .map(|g| g.members.iter().any(|id| id == agent) && g.files.iter().any(|id| id == file))
            .unwrap_or(false)
    }
    pub async fn request_guest(&self, agent: &str, v: Value) -> Result<Value, String> {
        self.change(|d| {
            let worker = v["worker"].as_str().ok_or("Choose a machine")?.to_string();
            if !d.workers.get(&worker).map(|w| !w.revoked).unwrap_or(false) {
                return Err("Machine not found".into());
            }
            let path = normalize_scope(v["path"].as_str().unwrap_or(""))?;
            let reason = v["reason"]
                .as_str()
                .filter(|s| !s.trim().is_empty() && s.len() <= 1000)
                .ok_or("Explain why inspection is needed")?
                .to_string();
            if d.guests
                .values()
                .filter(|g| g.agent == agent && g.status == "pending")
                .count()
                >= 10
            {
                return Err("Too many pending requests".into());
            }
            let id = uuid::Uuid::new_v4().to_string();
            let g = Guest {
                id: id.clone(),
                agent: agent.into(),
                worker,
                path,
                reason,
                status: "pending".into(),
                expires_at: 0,
                approved_by: None,
                group: None,
            };
            let result = serde_json::to_value(&g).unwrap();
            d.guests.insert(id, g);
            Ok(result)
        })
        .await
    }
    pub async fn guest_requests(&self, agent: Option<&str>, worker: Option<&str>) -> Value {
        let d = self.data.lock().await;
        json!(d
            .guests
            .values()
            .filter(|g| agent.map(|a| g.agent == a).unwrap_or(true)
                && worker.map(|w| g.worker == w).unwrap_or(true))
            .collect::<Vec<_>>())
    }
    pub async fn approve_guest(
        &self,
        id: &str,
        owner: Option<&str>,
        approved: bool,
    ) -> Result<Value, String> {
        self.change(|d| {
            let g = d.guests.get_mut(id).ok_or("Request not found")?;
            if owner.map(|w| g.worker != w).unwrap_or(false) {
                return Err("This machine owner cannot approve another machine's request".into());
            }
            g.status = if approved { "approved" } else { "denied" }.into();
            g.expires_at = if approved { now() + 900 } else { 0 };
            g.approved_by = Some(owner.unwrap_or("sysadmin").into());
            Ok(serde_json::to_value(g).unwrap())
        })
        .await
    }
    pub async fn guest_task(&self, agent: &str, id: &str, v: Value) -> Result<Value, String> {
        self.change(|d| {
            let g = d
                .guests
                .get(id)
                .filter(|g| {
                    g.agent == agent
                        && g.status == "approved"
                        && g.expires_at > now()
                        && g.group
                            .as_ref()
                            .map(|id| {
                                d.groups
                                    .get(id)
                                    .map(|group| group.members.contains(&agent.to_string()))
                                    .unwrap_or(false)
                            })
                            .unwrap_or(true)
                })
                .ok_or("Read-only guest grant is missing, revoked or expired")?
                .clone();
            let path = normalize_scope(v["path"].as_str().unwrap_or(&g.path))?;
            if !g.path.is_empty() && path != g.path && !path.starts_with(&(g.path.clone() + "/")) {
                return Err("Path is outside the approved folder".into());
            }
            let kind = v["kind"].as_str().unwrap_or("list_files");
            if !["list_files", "fetch_file"].contains(&kind) {
                return Err("Guest access is read-only".into());
            }
            if d.tasks
                .values()
                .filter(|t| {
                    t.status == "queued" && t.request_id.starts_with(&format!("guest:{agent}:"))
                })
                .count()
                >= 20
            {
                return Err("Guest inspection queue is full".into());
            }
            let tid = uuid::Uuid::new_v4().to_string();
            let t = Task {
                id: tid.clone(),
                title: format!("Guest inspection: {path}"),
                kind: kind.into(),
                target: g.worker,
                argv: vec![],
                path,
                inputs: vec![],
                status: "queued".into(),
                created_at: now(),
                updated_at: now(),
                timeout_seconds: 300,
                request_id: format!("guest:{agent}:{id}"),
                worker_id: None,
                lease: None,
                lease_until: 0,
                agent_id: None,
                agent_token: None,
                result: json!({}),
                outputs: vec![],
                error: None,
            };
            let result = public_task(&t);
            d.tasks.insert(tid, t);
            Ok(result)
        })
        .await
    }
    pub async fn agent_task(&self, agent: &str, id: &str) -> Result<Value, String> {
        let d = self.data.lock().await;
        let t = d.tasks.get(id).ok_or("Task not found")?;
        // Binding is encoded independently of worker-controlled result data below.
        let binding = t
            .request_id
            .strip_prefix("guest:")
            .and_then(|s| s.split_once(':'));
        if !binding
            .map(|(a, g)| {
                a == agent
                    && d.guests
                        .get(g)
                        .map(|g| {
                            g.agent == agent
                                && g.status == "approved"
                                && g.expires_at > now()
                                && g.group
                                    .as_ref()
                                    .map(|id| {
                                        d.groups
                                            .get(id)
                                            .map(|group| group.members.contains(&agent.to_string()))
                                            .unwrap_or(false)
                                    })
                                    .unwrap_or(true)
                        })
                        .unwrap_or(false)
            })
            .unwrap_or(false)
        {
            return Err("Task result access denied".into());
        }
        Ok(public_task(t))
    }
    pub async fn owner_auth(&self, id: &str, key: &str) -> bool {
        if key.len() < 64 {
            return false;
        }
        let hash = crate::admin::hash(key);
        self.data
            .lock()
            .await
            .workers
            .get(id)
            .map(|w| !w.revoked && constant_eq(&w.owner_hash, &hash))
            .unwrap_or(false)
    }
}
fn normalize_scope(path: &str) -> Result<String, String> {
    if path.len() > 4096
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path.split('/').any(|s| s == ".." || s.contains(':'))
    {
        return Err("Use a relative path inside the machine's shared folder".into());
    }
    Ok(path
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect::<Vec<_>>()
        .join("/"))
}

fn guest_valid(d: &Data, t: &Task) -> bool {
    if let Some(binding) = t.request_id.strip_prefix("guest:") {
        let Some((agent, grant)) = binding.split_once(':') else {
            return false;
        };
        return d.agents.get(agent).map(|a| !a.revoked).unwrap_or(false)
            && d.guests
                .get(grant)
                .map(|g| {
                    g.agent == agent
                        && g.status == "approved"
                        && g.expires_at > now()
                        && g.group
                            .as_ref()
                            .map(|id| {
                                d.groups
                                    .get(id)
                                    .map(|group| group.members.contains(&agent.to_string()))
                                    .unwrap_or(false)
                            })
                            .unwrap_or(true)
                })
                .unwrap_or(false);
    }
    true
}

impl Cloud {
    pub async fn group_members(&self, id: &str, members: Vec<String>) -> Result<Value, String> {
        self.change(|d| {
            if members.len() > 100
                || members
                    .iter()
                    .any(|id| !d.agents.get(id).map(|a| !a.revoked).unwrap_or(false))
            {
                return Err("Choose active agents".into());
            }
            d.groups.get_mut(id).ok_or("Group not found")?.members = members;
            Ok(json!({"updated":true}))
        })
        .await
    }
    pub async fn group_folder(&self, id: &str, worker: &str, path: &str) -> Result<Value, String> {
        let path = normalize_scope(path)?;
        self.change(|d| {
            if !d.workers.get(worker).map(|w| !w.revoked).unwrap_or(false) {
                return Err("Worker not found".into());
            }
            let group = d.groups.get_mut(id).ok_or("Group not found")?;
            let folder = json!({"id":uuid::Uuid::new_v4().to_string(),"worker":worker,"path":path});
            group.folders.push(folder.clone());
            Ok(folder)
        })
        .await
    }
    pub async fn remove_group_share(&self, id: &str, item: &str) -> Result<Value, String> {
        self.change(|d| {
            let group = d.groups.get_mut(id).ok_or("Group not found")?;
            group.files.retain(|f| f != item);
            group.folders.retain(|f| f["id"].as_str() != Some(item));
            for g in d
                .guests
                .values_mut()
                .filter(|g| g.group.as_deref() == Some(id))
            {
                g.status = "revoked".into();
            }
            Ok(json!({"removed":true}))
        })
        .await
    }
    pub async fn group_folder_task(
        &self,
        agent: &str,
        group: &str,
        folder: &str,
        v: Value,
    ) -> Result<Value, String> {
        let scope = {
            let d = self.data.lock().await;
            let g = d
                .groups
                .get(group)
                .filter(|g| g.members.iter().any(|id| id == agent))
                .ok_or("Group access denied")?;
            g.folders
                .iter()
                .find(|f| f["id"].as_str() == Some(folder))
                .cloned()
                .ok_or("Group folder not found")?
        };
        let grant=self.request_guest(agent,json!({"worker":scope["worker"],"path":scope["path"],"reason":"Inspect explicitly shared group folder"})).await?;
        let id = grant["id"].as_str().unwrap();
        self.change(|d| {
            if !d
                .groups
                .get(group)
                .map(|g| {
                    g.members.contains(&agent.to_string())
                        && g.folders.iter().any(|f| f["id"].as_str() == Some(folder))
                })
                .unwrap_or(false)
            {
                return Err("Group folder access was removed".into());
            }
            let g = d.guests.get_mut(id).unwrap();
            g.group = Some(group.into());
            g.status = "approved".into();
            g.expires_at = now() + 900;
            g.approved_by = Some("group policy".into());
            Ok(())
        })
        .await?;
        self.guest_task(agent, id, v).await
    }
}

impl Cloud {
    pub async fn managed_agent(&self, v: Value) -> Result<Value, String> {
        let name = v["name"]
            .as_str()
            .filter(|s| !s.trim().is_empty() && s.len() <= 100)
            .ok_or("Agent name required")?
            .to_string();
        let Some(worker) = v["worker"].as_str().filter(|w| !w.is_empty()) else {
            return self.create_agent(name).await;
        };
        let argv: Vec<String> = serde_json::from_value(v["argv"].clone())
            .map_err(|_| "Agent command must be an argument array")?;
        if argv.is_empty()
            || argv.len() > 128
            || argv.iter().any(|s| s.len() > 8192 || s.contains('\0'))
        {
            return Err("Invalid agent command".into());
        }
        self.change(|d| {
            if !d.workers.get(worker).map(|w| !w.revoked).unwrap_or(false) {
                return Err("Choose an approved worker".into());
            }
            let id = uuid::Uuid::new_v4().to_string();
            let token = secret();
            let tid = uuid::Uuid::new_v4().to_string();
            d.agents.insert(
                id.clone(),
                AgentIdentity {
                    id: id.clone(),
                    name: name.clone(),
                    token_hash: crate::admin::hash(&token),
                    revoked: false,
                    worker: Some(worker.into()),
                    task_id: Some(tid.clone()),
                },
            );
            d.tasks.insert(
                tid.clone(),
                Task {
                    id: tid,
                    title: format!("Agent: {name}"),
                    kind: "command".into(),
                    target: worker.into(),
                    argv,
                    path: String::new(),
                    inputs: vec![],
                    status: "queued".into(),
                    created_at: now(),
                    updated_at: now(),
                    timeout_seconds: 3600,
                    request_id: secret(),
                    worker_id: None,
                    lease: None,
                    lease_until: 0,
                    agent_id: Some(id.clone()),
                    agent_token: Some(token.clone()),
                    result: json!({}),
                    outputs: vec![],
                    error: None,
                },
            );
            Ok(json!({"agent_id":id,"token":token}))
        })
        .await
    }
}

fn clean_resources(v: Value) -> Value {
    let mut clean = serde_json::Map::new();
    for key in [
        "cpu_cores",
        "ram_total_bytes",
        "ram_available_bytes",
        "disk_free_bytes",
        "disk_total_bytes",
    ] {
        if let Some(value) = v[key].as_u64() {
            clean.insert(key.into(), json!(value));
        }
    }
    for key in ["hostname", "platform", "shared_folder"] {
        if let Some(value) = v[key].as_str().filter(|s| s.len() <= 4096) {
            clean.insert(key.into(), json!(value));
        }
    }
    Value::Object(clean)
}
fn clean_result(v: Value) -> Result<Value, String> {
    let success = v["success"]
        .as_bool()
        .ok_or("Task result requires a success flag")?;
    let mut clean = serde_json::Map::new();
    clean.insert("success".into(), json!(success));
    if let Some(truncated) = v["truncated"].as_bool() {
        clean.insert("truncated".into(), json!(truncated));
    }
    for key in ["stdout", "stderr", "error", "path", "file_id"] {
        if let Some(text) = v[key].as_str() {
            if text.len() > 65536 {
                return Err("Task result text exceeds 64 KB".into());
            }
            clean.insert(key.into(), json!(text));
        }
    }
    if let Some(code) = v["exit_code"].as_i64() {
        clean.insert("exit_code".into(), json!(code));
    }
    if let Some(entries) = v["files"].as_array() {
        if entries.len() > 1000 {
            return Err("Directory result exceeds 1000 entries".into());
        }
        let mut files = vec![];
        for entry in entries {
            let name = entry["name"]
                .as_str()
                .filter(|s| s.len() <= 4096)
                .ok_or("Invalid directory entry")?;
            let path = entry["path"]
                .as_str()
                .filter(|s| s.len() <= 4096)
                .ok_or("Invalid directory entry path")?;
            files.push(json!({"name":name,"path":path,"is_dir":entry["is_dir"].as_bool().unwrap_or(false),"size_bytes":entry["size_bytes"].as_u64().unwrap_or(0)}));
        }
        clean.insert("files".into(), json!(files));
    }
    Ok(Value::Object(clean))
}

impl Cloud {
    pub async fn group_records(
        &self,
        group: &str,
        agent: Option<&str>,
        after: usize,
    ) -> Result<Value, String> {
        let d = self.data.lock().await;
        let g = d
            .groups
            .get(group)
            .filter(|g| {
                agent
                    .map(|a| g.members.iter().any(|id| id == a))
                    .unwrap_or(true)
            })
            .ok_or("Group access denied")?;
        Ok(
            json!({"messages":g.messages.iter().enumerate().skip(after).take(100).map(|(i,m)|json!({"sequence":i,"record":m})).collect::<Vec<_>>(),"next":(after+100).min(g.messages.len()),"total":g.messages.len()}),
        )
    }
}

impl Cloud {
    pub async fn task_record(&self, id: &str) -> Result<Value, String> {
        self.data
            .lock()
            .await
            .tasks
            .get(id)
            .map(public_task)
            .ok_or("Task record not found".into())
    }
}
impl Cloud {
    pub async fn history(&self, offset: usize) -> Value {
        let d = self.data.lock().await;
        let mut tasks: Vec<_> = d.tasks.values().collect();
        tasks.sort_by_key(|t| {
            (
                std::cmp::Reverse(t.created_at),
                std::cmp::Reverse(t.id.as_str()),
            )
        });
        json!({"tasks":tasks.into_iter().skip(offset).take(100).map(|t|{let mut v=public_task(t);v["result"]=json!({});v}).collect::<Vec<_>>(),"next":offset.saturating_add(100).min(d.tasks.len()),"total":d.tasks.len()})
    }
}
