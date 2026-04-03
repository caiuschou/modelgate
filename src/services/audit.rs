use std::sync::Arc;
use std::{fs, path::Path};

use crate::audit::{
    now_unix_millis, AuditListItem, AuditListQuery, AuditRecord, ExportRequest, ExportResponse,
    ExportStatusResponse,
};

use super::error::ServiceError;
use super::repository::Repository;

pub trait AuditService: Send + Sync {
    fn list_audit_logs(
        &self,
        query: &AuditListQuery,
        user_id: i64,
    ) -> Result<(Vec<AuditListItem>, i64), ServiceError>;
    fn get_audit_log(&self, request_id: &str, user_id: i64) -> Result<AuditRecord, ServiceError>;
    fn export_audit_logs(
        &self,
        user_id: i64,
        payload: &ExportRequest,
        export_dir: &str,
    ) -> Result<ExportResponse, ServiceError>;
    fn get_export_status(
        &self,
        export_id: &str,
        export_dir: &str,
    ) -> Result<ExportStatusResponse, ServiceError>;
    fn download_export_file(
        &self,
        export_id: &str,
        export_dir: &str,
    ) -> Result<ExportFileData, ServiceError>;
}

pub struct DefaultAuditService {
    repo: Arc<dyn Repository>,
}

pub struct ExportFileData {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub file_name: String,
}

impl DefaultAuditService {
    pub fn new(repo: Arc<dyn Repository>) -> Self {
        Self { repo }
    }
}

impl AuditService for DefaultAuditService {
    fn list_audit_logs(
        &self,
        query: &AuditListQuery,
        user_id: i64,
    ) -> Result<(Vec<AuditListItem>, i64), ServiceError> {
        self.repo
            .query_audit_logs(query, Some(user_id))
            .map_err(ServiceError::from)
    }

    fn get_audit_log(&self, request_id: &str, user_id: i64) -> Result<AuditRecord, ServiceError> {
        self.repo
            .get_audit_log_by_request_id(request_id, Some(user_id))
            .map_err(ServiceError::from)
    }

    fn export_audit_logs(
        &self,
        user_id: i64,
        payload: &ExportRequest,
        export_dir: &str,
    ) -> Result<ExportResponse, ServiceError> {
        let format = payload
            .format
            .clone()
            .unwrap_or_else(|| "json".to_string())
            .to_lowercase();
        if format != "json" && format != "csv" {
            return Err(ServiceError::BadRequest(
                "format must be either `json` or `csv`".into(),
            ));
        }

        let mut offset = 0_u32;
        let page_size = 1000_u32;
        let mut all_rows = Vec::new();
        loop {
            let query = AuditListQuery {
                start_time: payload.start_time,
                end_time: payload.end_time,
                user_id: None,
                token_id: None,
                channel_id: None,
                model: None,
                status_code: None,
                keyword: None,
                app_id: None,
                finish_reason: None,
                min_prompt_tokens: None,
                max_prompt_tokens: None,
                min_completion_tokens: None,
                max_completion_tokens: None,
                limit: Some(page_size),
                offset: Some(offset),
            };
            let (rows, _total) = self
                .repo
                .query_audit_logs(&query, Some(user_id))
                .map_err(ServiceError::from)?;
            let count = rows.len() as u32;
            all_rows.extend(rows);
            if count < page_size {
                break;
            }
            offset = offset.saturating_add(page_size);
        }

        let export_id = format!("exp_{}", now_unix_millis());
        let ext = if format == "csv" { "csv" } else { "json" };
        let file_name = format!("{export_id}.{ext}");
        let file_path = Path::new(export_dir).join(&file_name);

        if format == "csv" {
            fs::write(&file_path, build_csv_content(&all_rows))
                .map_err(|_| ServiceError::Internal("Failed to write export file".into()))?;
        } else {
            let json_content = serde_json::to_vec_pretty(&all_rows)
                .map_err(|_| ServiceError::Internal("Failed to serialize export json".into()))?;
            fs::write(&file_path, json_content)
                .map_err(|_| ServiceError::Internal("Failed to write export file".into()))?;
        }

        Ok(ExportResponse {
            export_id: export_id.clone(),
            status: "success".to_string(),
            download_url: format!("/api/v1/logs/export/{export_id}/download"),
        })
    }

    fn get_export_status(
        &self,
        export_id: &str,
        export_dir: &str,
    ) -> Result<ExportStatusResponse, ServiceError> {
        let status = if find_export_file(export_dir, export_id).is_some() {
            "success"
        } else {
            "processing"
        };
        Ok(ExportStatusResponse {
            export_id: export_id.to_string(),
            status: status.to_string(),
        })
    }

    fn download_export_file(
        &self,
        export_id: &str,
        export_dir: &str,
    ) -> Result<ExportFileData, ServiceError> {
        let file_path = find_export_file(export_dir, export_id)
            .ok_or_else(|| ServiceError::NotFound("export file not found".into()))?;
        let bytes = fs::read(&file_path)
            .map_err(|_| ServiceError::Internal("Failed to read export file".into()))?;
        let content_type = if file_path
            .extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("csv"))
            .unwrap_or(false)
        {
            "text/csv; charset=utf-8"
        } else {
            "application/json; charset=utf-8"
        };
        let file_name = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("audit-export")
            .to_string();
        Ok(ExportFileData {
            bytes,
            content_type: content_type.to_string(),
            file_name,
        })
    }
}

fn find_export_file(export_dir: &str, export_id: &str) -> Option<std::path::PathBuf> {
    let iter = fs::read_dir(export_dir).ok()?;
    for entry in iter.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str());
        if stem == Some(export_id) {
            return Some(path);
        }
    }
    None
}

fn escape_csv(value: &str) -> String {
    let escaped = value.replace('\"', "\"\"");
    format!("\"{escaped}\"")
}

fn build_csv_content(rows: &[AuditListItem]) -> String {
    let mut out = String::from(
        "request_id,user_id,token_id,channel_id,model,request_type,status_code,error_message,prompt_tokens,completion_tokens,total_tokens,cost,latency_ms,app_id,finish_reason,created_at\n",
    );
    for row in rows {
        let line = format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            escape_csv(&row.request_id),
            row.user_id.map(|v| v.to_string()).unwrap_or_default(),
            row.token_id.map(|v| v.to_string()).unwrap_or_default(),
            row.channel_id
                .as_deref()
                .map(escape_csv)
                .unwrap_or_default(),
            row.model.as_deref().map(escape_csv).unwrap_or_default(),
            row.request_type
                .as_deref()
                .map(escape_csv)
                .unwrap_or_default(),
            row.status_code.map(|v| v.to_string()).unwrap_or_default(),
            row.error_message
                .as_deref()
                .map(escape_csv)
                .unwrap_or_default(),
            row.prompt_tokens.map(|v| v.to_string()).unwrap_or_default(),
            row.completion_tokens
                .map(|v| v.to_string())
                .unwrap_or_default(),
            row.total_tokens.map(|v| v.to_string()).unwrap_or_default(),
            row.cost.map(|v| v.to_string()).unwrap_or_default(),
            row.latency_ms.map(|v| v.to_string()).unwrap_or_default(),
            row.app_id.as_deref().map(escape_csv).unwrap_or_default(),
            row.finish_reason
                .as_deref()
                .map(escape_csv)
                .unwrap_or_default(),
            row.created_at
        );
        out.push_str(&line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::error::RepositoryError;
    use std::sync::Mutex;

    struct MockRepo {
        query_calls: Mutex<u32>,
        rows: Vec<AuditListItem>,
    }

    impl Repository for MockRepo {
        fn get_api_key_info(&self, _api_key: &str) -> Result<(i64, i64), RepositoryError> {
            Err(RepositoryError::NotFound("api key not found".into()))
        }
        fn query_audit_logs(
            &self,
            _query: &AuditListQuery,
            _scoped_user_id: Option<i64>,
        ) -> Result<(Vec<AuditListItem>, i64), RepositoryError> {
            let mut calls = self.query_calls.lock().expect("lock query calls");
            *calls += 1;
            if *calls == 1 {
                Ok((self.rows.clone(), self.rows.len() as i64))
            } else {
                Ok((Vec::new(), self.rows.len() as i64))
            }
        }
        fn get_audit_log_by_request_id(
            &self,
            _request_id: &str,
            _scoped_user_id: Option<i64>,
        ) -> Result<AuditRecord, RepositoryError> {
            Err(RepositoryError::NotFound("audit log not found".into()))
        }
        fn create_user_with_api_key(
            &self,
            _username: &str,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
        fn create_api_key_for_user(
            &self,
            _username: &str,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
        fn register_user_with_password_and_api_key(
            &self,
            _username: &str,
            _password_hash: &str,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
        fn get_user_login_credentials(
            &self,
            _username: &str,
        ) -> Result<Option<(i64, Option<String>)>, RepositoryError> {
            Ok(None)
        }
        fn get_first_api_key_for_user(
            &self,
            _user_id: i64,
        ) -> Result<Option<String>, RepositoryError> {
            Ok(None)
        }
        fn create_api_key_for_user_id(
            &self,
            _user_id: i64,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
        fn list_api_keys_for_user(
            &self,
            _user_id: i64,
        ) -> Result<Vec<crate::services::repository::ApiKeySummary>, RepositoryError> {
            Ok(Vec::new())
        }
        fn insert_api_key_for_user_returning_id(
            &self,
            _user_id: i64,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<i64, RepositoryError> {
            Ok(1)
        }
        fn revoke_api_key_for_user(
            &self,
            _user_id: i64,
            _key_id: i64,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
    }

    #[test]
    fn export_writes_json_file() {
        let repo = Arc::new(MockRepo {
            query_calls: Mutex::new(0),
            rows: vec![AuditListItem {
                request_id: "r1".into(),
                user_id: Some(1),
                token_id: Some(2),
                channel_id: None,
                model: Some("gpt-test".into()),
                request_type: Some("chat".into()),
                status_code: Some(200),
                error_message: None,
                prompt_tokens: Some(1),
                completion_tokens: Some(1),
                total_tokens: Some(2),
                cost: Some(0.01),
                latency_ms: Some(10),
                app_id: None,
                finish_reason: Some("stop".into()),
                created_at: 1,
            }],
        });
        let service = DefaultAuditService::new(repo);
        let temp_dir =
            std::env::temp_dir().join(format!("modelgate_export_test_{}", now_unix_millis()));
        std::fs::create_dir_all(&temp_dir).expect("create temp export dir");
        let req = ExportRequest {
            start_time: None,
            end_time: None,
            format: Some("json".into()),
        };
        let resp = service
            .export_audit_logs(1, &req, temp_dir.to_string_lossy().as_ref())
            .expect("export logs");
        assert_eq!(resp.status, "success");
        let id = resp
            .download_url
            .split('/')
            .nth_back(1)
            .expect("export id in download url");
        let file = find_export_file(temp_dir.to_string_lossy().as_ref(), id);
        assert!(file.is_some());
    }
}
