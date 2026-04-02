use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::error;

#[derive(Debug, Clone, Deserialize)]
pub struct AuditConfig {
    pub log_dir: String,
    pub retention_days: u32,
    pub batch_size: usize,
    pub flush_interval_seconds: u64,
    pub export_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub request_id: String,
    pub user_id: Option<i64>,
    pub token_id: Option<i64>,
    pub channel_id: Option<String>,
    pub model: Option<String>,
    pub request_type: Option<String>,
    pub request_body_path: Option<String>,
    pub response_body_path: Option<String>,
    pub status_code: Option<i64>,
    pub error_message: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cost: Option<f64>,
    pub latency_ms: Option<i64>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub enum AuditMessage {
    Record(AuditRecord),
}

#[derive(Debug, Deserialize)]
pub struct AuditListQuery {
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub user_id: Option<i64>,
    pub token_id: Option<i64>,
    pub channel_id: Option<String>,
    pub model: Option<String>,
    pub status_code: Option<i64>,
    pub keyword: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditListItem {
    pub request_id: String,
    pub user_id: Option<i64>,
    pub token_id: Option<i64>,
    pub channel_id: Option<String>,
    pub model: Option<String>,
    pub request_type: Option<String>,
    pub status_code: Option<i64>,
    pub error_message: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cost: Option<f64>,
    pub latency_ms: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct AuditListResponse {
    pub data: Vec<AuditListItem>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub export_id: String,
    pub status: String,
    pub download_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub format: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportStatusResponse {
    pub export_id: String,
    pub status: String,
}

pub fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn now_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub fn generate_request_id() -> String {
    let ts = now_unix_millis();
    let random: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    format!("{ts}_{random}")
}

pub fn save_body_to_file(
    cfg: &AuditConfig,
    request_id: &str,
    body_type: &str,
    body: &[u8],
) -> std::io::Result<String> {
    let now = now_unix_secs();
    let month_bucket = now / (30 * 24 * 3600);
    let dir = Path::new(&cfg.log_dir).join(format!("{month_bucket}"));
    fs::create_dir_all(&dir)?;
    let file = dir.join(format!("{request_id}-{body_type}.json"));
    fs::write(&file, body)?;
    Ok(path_to_string(&file))
}

pub fn ensure_storage_dirs(cfg: &AuditConfig) -> std::io::Result<()> {
    fs::create_dir_all(&cfg.log_dir)?;
    fs::create_dir_all(&cfg.export_dir)?;
    Ok(())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub async fn audit_writer_loop(
    mut receiver: mpsc::Receiver<AuditMessage>,
    db: crate::db::DbConn,
    config: AuditConfig,
) {
    let mut buffer: Vec<AuditRecord> = Vec::new();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(
        config.flush_interval_seconds,
    ));

    loop {
        let mut should_flush = false;
        tokio::select! {
            msg = receiver.recv() => {
                match msg {
                    Some(AuditMessage::Record(record)) => {
                        buffer.push(record);
                        if buffer.len() >= config.batch_size {
                            should_flush = true;
                        }
                    }
                    None => {
                        if !buffer.is_empty() {
                            let mut conn = match db.get() {
                                Ok(conn) => conn,
                                Err(err) => {
                                    error!(error = %err, "failed to get db connection from pool");
                                    break;
                                }
                            };
                            if let Err(err) = crate::db::insert_audit_logs(&mut conn, &buffer) {
                                error!(error = %err, "audit flush failed on channel close");
                            }
                            buffer.clear();
                        }
                        break;
                    }
                }
            }
            _ = interval.tick() => {
                should_flush = !buffer.is_empty();
            }
        }

        if should_flush && !buffer.is_empty() {
            let mut conn = match db.get() {
                Ok(conn) => conn,
                Err(err) => {
                    error!(error = %err, "failed to get db connection from pool");
                    continue;
                }
            };
            if let Err(err) = crate::db::insert_audit_logs(&mut conn, &buffer) {
                error!(error = %err, "audit batch insert failed");
            }
            buffer.clear();
        }
    }
}
