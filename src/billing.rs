//! Prepaid balance (USD minor units, k=15) and charges from **upstream-reported USD** when `billing.enabled` is true.
//! **BYOK** (`is_byok`): user pays their own upstream; platform balance is not checked and not charged.

use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;

use crate::{db, errors::ApiError, money, AppState};

/// Prefer `upstream_inference_cost` in `cost_details`, else sum of `cost_details` numerics, else `cost`.
/// Checks the root object and then **`usage`** (OpenRouter / stream chunks put `cost` under `usage`).
pub fn upstream_cost_usd_from_response(value: &Value) -> Option<Decimal> {
    upstream_cost_usd_from_object(value)
        .or_else(|| value.get("usage").and_then(upstream_cost_usd_from_object))
}

fn upstream_cost_usd_from_object(obj: &Value) -> Option<Decimal> {
    if let Some(cd) = obj.get("cost_details").and_then(|v| v.as_object()) {
        if let Some(v) = cd.get("upstream_inference_cost") {
            if let Some(d) = json_value_to_decimal(v) {
                if d > Decimal::ZERO {
                    return Some(d);
                }
            }
        }
        let mut sum = Decimal::ZERO;
        for (_k, vv) in cd {
            if let Some(d) = json_value_to_decimal(vv) {
                sum += d;
            }
        }
        if sum > Decimal::ZERO {
            return Some(sum);
        }
    }
    obj.get("cost").and_then(json_value_to_decimal)
}

fn json_value_to_decimal(v: &Value) -> Option<Decimal> {
    match v {
        Value::Number(n) => Decimal::from_str(&n.to_string()).ok(),
        Value::String(s) => Decimal::from_str(s.trim()).ok(),
        _ => None,
    }
}

/// Blocks chat when balance is empty (only if billing is enabled).
/// Skips when `is_byok` — the caller uses their own upstream key; no platform balance applies.
pub fn check_can_start_chat(state: &AppState, user_id: i64, is_byok: bool) -> Result<(), ApiError> {
    if is_byok {
        return Ok(());
    }
    if !state.cfg.billing.enabled {
        return Ok(());
    }
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let bal = db::get_balance_minor(&conn, user_id)
        .map_err(|e| ApiError::InternalError(format!("billing balance: {e}")))?;
    if bal <= 0 {
        return Err(ApiError::InsufficientBalance {
            message: "Account balance is zero. Add funds to use the API.".into(),
            balance_minor: bal,
        });
    }
    Ok(())
}

/// Records usage charge after a successful upstream response from upstream-reported USD.
/// Skips when `is_byok` — no platform charge for BYOK traffic.
#[allow(clippy::too_many_arguments)] // thin wrapper over DB charge + tracing
pub fn charge_chat_usage(
    state: &AppState,
    user_id: i64,
    gateway_key_id: i64,
    request_id: &str,
    model: Option<&str>,
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    upstream_usd: Option<Decimal>,
    is_byok: bool,
) {
    if is_byok {
        return;
    }
    if !state.cfg.billing.enabled {
        return;
    }
    let charge_minor = match upstream_usd {
        Some(usd) if usd > Decimal::ZERO => money::usd_to_minor(usd),
        _ => {
            tracing::warn!(
                %request_id,
                "billing: no upstream USD cost in response; charging 0 minor units"
            );
            0
        }
    };
    let now = crate::audit::now_unix_secs();
    let Ok(conn) = state.db.get() else {
        tracing::error!("billing: database pool unavailable for charge");
        return;
    };
    match db::billing_charge_usage(
        &conn,
        user_id,
        Some(gateway_key_id),
        charge_minor,
        now,
        db::BillingUsageChargeMeta {
            request_id,
            model,
            prompt_tokens,
            completion_tokens,
        },
    ) {
        Ok(_) => {}
        Err(db::BillingChargeError::Insufficient { balance_minor }) => {
            tracing::warn!(
                %request_id,
                user_id,
                balance_minor,
                charge_minor,
                "billing charge skipped: insufficient balance after upstream success"
            );
        }
        Err(db::BillingChargeError::Database(e)) => {
            tracing::error!(%request_id, error = %e, "billing charge database error");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn upstream_cost_prefers_inference_total() {
        let v = json!({
            "cost_details": {
                "upstream_inference_cost": "0.0000189",
                "upstream_inference_prompt_cost": "0.0000018",
                "upstream_inference_completions_cost": "0.0000171"
            }
        });
        let d = upstream_cost_usd_from_response(&v).unwrap();
        assert_eq!(d.to_string(), "0.0000189");
    }

    #[test]
    fn upstream_cost_sums_details_when_no_total() {
        let v = json!({
            "cost_details": {
                "upstream_inference_prompt_cost": "0.0000018",
                "upstream_inference_completions_cost": "0.0000171"
            }
        });
        let d = upstream_cost_usd_from_response(&v).unwrap();
        assert_eq!(d.to_string(), "0.0000189");
    }

    #[test]
    fn upstream_cost_top_level_cost() {
        let v = json!({ "cost": 0.01 });
        let d = upstream_cost_usd_from_response(&v).unwrap();
        assert_eq!(d.to_string(), "0.01");
    }

    /// OpenRouter-style: `cost` / `cost_details` live under `usage` (e.g. final SSE chunk).
    #[test]
    fn upstream_cost_nested_under_usage() {
        let v = json!({
            "object": "chat.completion.chunk",
            "usage": {
                "prompt_tokens": 42,
                "completion_tokens": 79,
                "total_tokens": 121,
                "cost": 0.0000252,
                "cost_details": {
                    "upstream_inference_cost": 0.0000252,
                    "upstream_inference_prompt_cost": 0.0000015,
                    "upstream_inference_completions_cost": 0.0000237
                }
            }
        });
        let d = upstream_cost_usd_from_response(&v).unwrap();
        assert_eq!(d.to_string(), "0.0000252");
    }

    #[test]
    fn upstream_cost_none_when_no_usable_amount() {
        let v = json!({ "cost_details": {} });
        assert!(upstream_cost_usd_from_response(&v).is_none());
    }
}
