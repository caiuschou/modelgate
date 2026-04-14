//! In-process broadcast for audit log changes (WebSocket subscribers filter by scope).

use serde::Serialize;
use tokio::sync::broadcast;

pub const AUDIT_LOG_UPDATED: &str = "audit_log_updated";

#[derive(Clone, Debug, Serialize)]
pub struct AuditNotifyPayload {
    pub v: u8,
    #[serde(rename = "type")]
    pub ty: &'static str,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_completed: Option<bool>,
}

#[derive(Clone)]
pub struct AuditNotifyHub {
    tx: broadcast::Sender<AuditNotifyPayload>,
}

impl AuditNotifyHub {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AuditNotifyPayload> {
        self.tx.subscribe()
    }

    pub fn notify(&self, payload: AuditNotifyPayload) {
        let _ = self.tx.send(payload);
    }
}

pub fn payload_from_record(record: &crate::audit::AuditRecord) -> AuditNotifyPayload {
    let stream_completed = record
        .metadata
        .as_ref()
        .and_then(|m| m.get("stream_completed"))
        .and_then(|v| v.as_bool());
    AuditNotifyPayload {
        v: 1,
        ty: AUDIT_LOG_UPDATED,
        request_id: record.request_id.clone(),
        user_id: record.user_id,
        team_id: record.team_id,
        thread_id: record.thread_id.clone(),
        stream_completed,
    }
}

pub fn payload_from_snapshot(
    request_id: String,
    s: crate::db::AuditNotifySnapshot,
) -> AuditNotifyPayload {
    AuditNotifyPayload {
        v: 1,
        ty: AUDIT_LOG_UPDATED,
        request_id,
        user_id: s.user_id,
        team_id: s.team_id,
        thread_id: s.thread_id,
        stream_completed: s.stream_completed,
    }
}

/// `ws_team` is `None` when the client opened the socket in **personal** console context
/// (no `X-Team-Id` / `team_id` query). Otherwise it is the team id for **team** context.
pub fn should_deliver_to_ws(
    p: &AuditNotifyPayload,
    viewer_user_id: i64,
    ws_team: Option<i64>,
) -> bool {
    let Some(row_uid) = p.user_id else {
        return false;
    };
    match ws_team {
        None => row_uid == viewer_user_id && p.team_id.is_none(),
        Some(tid) => p.team_id == Some(tid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(user_id: Option<i64>, team_id: Option<i64>) -> AuditNotifyPayload {
        AuditNotifyPayload {
            v: 1,
            ty: AUDIT_LOG_UPDATED,
            request_id: "r1".into(),
            user_id,
            team_id,
            thread_id: None,
            stream_completed: None,
        }
    }

    #[test]
    fn personal_ws_sees_only_personal_rows() {
        assert!(should_deliver_to_ws(&sample(Some(5), None), 5, None));
        assert!(!should_deliver_to_ws(&sample(Some(5), Some(1)), 5, None));
        assert!(!should_deliver_to_ws(&sample(Some(4), None), 5, None));
    }

    #[test]
    fn team_ws_sees_matching_team_only() {
        assert!(should_deliver_to_ws(&sample(Some(5), Some(9)), 5, Some(9)));
        assert!(!should_deliver_to_ws(&sample(Some(5), Some(8)), 5, Some(9)));
        assert!(!should_deliver_to_ws(&sample(Some(5), None), 5, Some(9)));
    }
}
