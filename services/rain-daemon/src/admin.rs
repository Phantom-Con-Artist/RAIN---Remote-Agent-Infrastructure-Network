//! High-entropy owner key -> short-lived, host-only HttpOnly session.
use hyper::HeaderMap;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::PathBuf};
use tokio::{io::AsyncWriteExt, sync::Mutex};
fn token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
pub(crate) fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
fn equal(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0, |v, (a, b)| v | (a ^ b)) == 0
}
struct Data {
    key_hash: String,
    sessions: HashMap<String, i64>,
    failures: u32,
    window: i64,
}
pub(crate) struct Admin {
    path: PathBuf,
    identity: Value,
    data: Mutex<Data>,
}
impl Admin {
    pub async fn restore(path: PathBuf) -> Result<Self, String> {
        if path.with_file_name("infrastructure.json").exists() && !path.exists() {
            return Err("Saved infrastructure is missing its admin key. Restore the private backup; authentication will not be reset automatically.".into());
        }
        let key = match tokio::fs::read_to_string(&path).await {
            Ok(key) => key.trim().to_string(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let key = token();
                let mut options = tokio::fs::OpenOptions::new();
                options.create_new(true).write(true);
                #[cfg(unix)]
                options.mode(0o600);
                let mut f = options.open(&path).await.map_err(|e| e.to_string())?;
                f.write_all(key.as_bytes())
                    .await
                    .map_err(|e| e.to_string())?;
                f.sync_all().await.map_err(|e| e.to_string())?;
                {
                    use std::io::IsTerminal;
                    if std::io::stdout().is_terminal() {
                        println!("Your sysadmin key (full control; keep private): {}", key);
                        println!("Admin key file: {}", path.display());
                    }
                }
                key
            }
            Err(e) => return Err(e.to_string()),
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = tokio::fs::metadata(&path)
                .await
                .map_err(|e| e.to_string())?
                .permissions()
                .mode();
            if permissions & 0o077 != 0 {
                return Err("Admin key file must be private (chmod 600 admin.key)".into());
            }
        }
        let identity_path = path.with_file_name("infrastructure.json");
        let identity: Value = match tokio::fs::read(&identity_path).await {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|_| "Saved infrastructure metadata is invalid")?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let v = json!({"id":uuid::Uuid::new_v4().to_string(),"name":std::env::var("RAIN_NODE_NAME").unwrap_or_else(|_|"My RAIN cloud".into()),"created_at":chrono::Utc::now().timestamp()});
                let mut options = tokio::fs::OpenOptions::new();
                options.create_new(true).write(true);
                #[cfg(unix)]
                options.mode(0o600);
                let mut f = options
                    .open(identity_path)
                    .await
                    .map_err(|e| e.to_string())?;
                f.write_all(&serde_json::to_vec(&v).unwrap())
                    .await
                    .map_err(|e| e.to_string())?;
                f.sync_all().await.map_err(|e| e.to_string())?;
                v
            }
            Err(e) => return Err(e.to_string()),
        };
        if !identity["id"].is_string() || !identity["name"].is_string() {
            return Err("Saved infrastructure requires an ID and name".into());
        }
        if key.len() < 64 {
            return Err("Admin key must contain at least 64 characters".into());
        }
        Ok(Self {
            path,
            identity,
            data: Mutex::new(Data {
                key_hash: hash(&key),
                sessions: HashMap::new(),
                failures: 0,
                window: chrono::Utc::now().timestamp(),
            }),
        })
    }
    pub fn identity(&self) -> Value {
        self.identity.clone()
    }
    pub async fn login(&self, key: &str) -> Result<String, String> {
        let mut d = self.data.lock().await;
        let now = chrono::Utc::now().timestamp();
        if now - d.window > 60 {
            d.failures = 0;
            d.window = now;
        }
        if !equal(&hash(key), &d.key_hash) {
            d.failures = d.failures.saturating_add(1);
            return Err(if d.failures > 20 {
                "Too many invalid login attempts. Use the key stored on the hub."
            } else {
                "Invalid admin key"
            }
            .into());
        }
        d.sessions.retain(|_, expiry| *expiry > now);
        if d.sessions.len() >= 100 {
            return Err("Session limit reached. Restart the hub to revoke sessions.".into());
        }
        let session = token();
        d.sessions.insert(hash(&session), now + 7200);
        Ok(session)
    }
    pub async fn authenticated(&self, headers: &HeaderMap) -> bool {
        let session = headers
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|cookie| {
                cookie
                    .split(';')
                    .find_map(|part| part.trim().strip_prefix("rain_admin="))
            });
        let Some(session) = session else {
            return false;
        };
        self.data
            .lock()
            .await
            .sessions
            .get(&hash(session))
            .map(|expiry| *expiry > chrono::Utc::now().timestamp())
            .unwrap_or(false)
    }
    pub async fn logout(&self, headers: &HeaderMap) {
        if let Some(key) = headers
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|c| {
                c.split(';')
                    .find_map(|p| p.trim().strip_prefix("rain_admin="))
            })
        {
            self.data.lock().await.sessions.remove(&hash(key));
        }
    }
    pub async fn audit(&self, action: &str, status: u16, role: &str) -> Result<(), String> {
        let _serial = self.data.lock().await;
        let path = self.path.with_file_name("admin-audit.jsonl");
        let mut options = tokio::fs::OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut f = options.open(path).await.map_err(|e| e.to_string())?;
        let mut record=serde_json::to_vec(&json!({"time":chrono::Utc::now().to_rfc3339(),"action":action,"status":status,"role":role})).map_err(|e|e.to_string())?;
        record.push(b'\n');
        f.write_all(&record).await.map_err(|e| e.to_string())?;
        f.sync_data().await.map_err(|e| e.to_string())?;
        Ok(())
    }
    pub async fn records(&self) -> Result<Value, String> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        let path = self.path.with_file_name("admin-audit.jsonl");
        let mut file = match tokio::fs::File::open(path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(json!([])),
            Err(e) => return Err(e.to_string()),
        };
        let length = file.metadata().await.map_err(|e| e.to_string())?.len();
        file.seek(std::io::SeekFrom::Start(length.saturating_sub(512 * 1024)))
            .await
            .map_err(|e| e.to_string())?;
        let mut bytes = vec![];
        file.take(512 * 1024)
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| e.to_string())?;
        let records: Vec<Value> = String::from_utf8_lossy(&bytes)
            .lines()
            .rev()
            .take(500)
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        Ok(json!(records))
    }
    pub async fn rotate(&self) -> Result<Value, String> {
        let key = token();
        let temp = self
            .path
            .with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let mut options = tokio::fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&temp).await.map_err(|e| e.to_string())?;
        file.write_all(key.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        file.sync_all().await.map_err(|e| e.to_string())?;
        drop(file);
        let mut d = self.data.lock().await;
        tokio::fs::rename(temp, &self.path)
            .await
            .map_err(|e| e.to_string())?;
        d.key_hash = hash(&key);
        d.sessions.clear();
        Ok(json!({"key":key,"sessions_revoked":true}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn key_sessions_logout_rotation_and_audit() {
        let folder = std::env::temp_dir().join(format!("rain-admin-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&folder).await.unwrap();
        let admin = Admin::restore(folder.join("admin.key")).await.unwrap();
        let key = tokio::fs::read_to_string(folder.join("admin.key"))
            .await
            .unwrap();
        assert!(admin.login("wrong").await.is_err());
        let session = admin.login(&key).await.unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("cookie", format!("rain_admin={session}").parse().unwrap());
        assert!(admin.authenticated(&headers).await);
        admin.logout(&headers).await;
        assert!(!admin.authenticated(&headers).await);
        let session = admin.login(&key).await.unwrap();
        headers.insert("cookie", format!("rain_admin={session}").parse().unwrap());
        let new = admin.rotate().await.unwrap();
        assert!(!admin.authenticated(&headers).await);
        assert!(admin.login(&key).await.is_err());
        assert!(admin.login(new["key"].as_str().unwrap()).await.is_ok());
        admin
            .audit("POST /api/v1/admin/login", 200, "sysadmin")
            .await
            .unwrap();
        let records = admin.records().await.unwrap();
        assert_eq!(records[0]["status"], 200);
        assert!(!records.to_string().contains(&key));
        tokio::fs::remove_file(folder.join("admin.key"))
            .await
            .unwrap();
        assert!(Admin::restore(folder.join("admin.key")).await.is_err());
        tokio::fs::remove_dir_all(folder).await.unwrap();
    }
}
