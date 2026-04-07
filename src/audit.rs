use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
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
    pub app_id: Option<String>,
    pub finish_reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
}

/// Second-phase update for streaming chat completions (response file + usage + metadata).
#[derive(Debug, Clone)]
pub struct AuditStreamCompletionUpdate {
    pub request_id: String,
    pub response_body_path: String,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cost: Option<f64>,
    pub finish_reason: Option<String>,
    pub latency_ms: i64,
    pub metadata: serde_json::Value,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AuditMessage {
    Record(AuditRecord),
    StreamCompletion(AuditStreamCompletionUpdate),
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
    /// 调用方应用标识（精确匹配）
    pub app_id: Option<String>,
    /// 多个值用英文逗号分隔，语义为 OR（如 `stop,length`）
    pub finish_reason: Option<String>,
    pub min_prompt_tokens: Option<i64>,
    pub max_prompt_tokens: Option<i64>,
    pub min_completion_tokens: Option<i64>,
    pub max_completion_tokens: Option<i64>,
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
    pub app_id: Option<String>,
    pub finish_reason: Option<String>,
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

/// Read an audit body file after resolving `stored_path` under `log_dir`. Rejects `..`
/// segments and paths over `max_bytes`.
///
/// `save_body_to_file` stores paths relative to the process working directory, typically
/// `<log_dir>/<bucket>/<request_id>-{request|response}.json` (the `log_dir` segment appears in
/// `stored_path`). Callers may also store paths relative to `log_dir` only (e.g. `685/id.json`).
/// We try `log_dir.join(stored)` first, then `stored` as-is relative to cwd.
pub fn read_audit_body_bytes(log_dir: &str, stored_path: &str) -> io::Result<Vec<u8>> {
    let log_root = Path::new(log_dir);
    let stored = Path::new(stored_path.trim());
    if stored.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "empty audit body path",
        ));
    }
    if stored.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "path escapes audit log directory",
        ));
    }

    let dir_canon = log_root.canonicalize().map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "audit log directory not available",
        )
    })?;

    let candidates: Vec<PathBuf> = if stored.is_absolute() {
        vec![stored.to_path_buf()]
    } else {
        vec![log_root.join(stored), stored.to_path_buf()]
    };

    let mut file_canon: Option<PathBuf> = None;
    for c in candidates {
        if let Ok(p) = c.canonicalize() {
            if p.starts_with(&dir_canon) {
                file_canon = Some(p);
                break;
            }
        }
    }

    let file_canon = file_canon.ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "audit body file missing")
    })?;

    fs::read(file_canon)
}

/// Create an empty response body file for streaming SSE (same layout as [`save_body_to_file`]).
pub async fn create_stream_response_body_file(
    cfg: &AuditConfig,
    request_id: &str,
) -> io::Result<(String, tokio::fs::File)> {
    let now = now_unix_secs();
    let month_bucket = now / (30 * 24 * 3600);
    let dir = Path::new(&cfg.log_dir).join(format!("{month_bucket}"));
    tokio::fs::create_dir_all(&dir).await?;
    let file_path = dir.join(format!("{request_id}-response.json"));
    let file = tokio::fs::File::create(&file_path).await?;
    Ok((path_to_string(&file_path), file))
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
                    Some(AuditMessage::StreamCompletion(update)) => {
                        if !buffer.is_empty() {
                            if let Ok(mut conn) = db.get() {
                                if let Err(err) = crate::db::insert_audit_logs(&mut conn, &buffer) {
                                    error!(error = %err, "audit flush before stream completion failed");
                                }
                            }
                            buffer.clear();
                        }
                        match db.get() {
                            Ok(mut conn) => {
                                if let Err(err) =
                                    crate::db::update_audit_log_stream_completion(&mut conn, &update)
                                {
                                    error!(error = %err, %update.request_id, "audit stream completion update failed");
                                }
                            }
                            Err(err) => {
                                error!(error = %err, "failed to get sqlite connection for stream completion");
                            }
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

#[cfg(test)]
mod read_body_tests {
    use super::*;

    #[test]
    fn read_rejects_parent_dir_components() {
        let tmp = std::env::temp_dir().join(format!("mg_audit_parent_{}", now_unix_millis()));
        fs::create_dir_all(&tmp).unwrap();
        let err = read_audit_body_bytes(&tmp.to_string_lossy(), "../outside").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_relative_file_under_log_dir() {
        let tmp = std::env::temp_dir().join(format!("mg_audit_rel_{}", now_unix_millis()));
        fs::create_dir_all(tmp.join("0")).unwrap();
        let rel = "0/xyz-request.json";
        fs::write(tmp.join(rel), br#"{"ok":true}"#).unwrap();
        let got = read_audit_body_bytes(&tmp.to_string_lossy(), rel).unwrap();
        assert_eq!(got, br#"{"ok":true}"#.as_slice());
        let _ = fs::remove_dir_all(&tmp);
    }
}
