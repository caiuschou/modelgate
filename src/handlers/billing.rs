use actix_web::http::header;
use actix_web::{web, HttpRequest, HttpResponse};
use rusqlite::Error as RusqliteError;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;
use subtle::ConstantTimeEq;

use crate::db;
use crate::errors::ApiError;
use crate::money;
use crate::session_auth;
use crate::AppState;

fn auth_user_id(req: &HttpRequest, state: &web::Data<AppState>) -> Result<i64, ApiError> {
    Ok(session_auth::resolve_console_session(req, state)?.user_id)
}

#[derive(Serialize)]
pub struct BalanceResponse {
    /// Integer minor units (scale k=15); no floating point.
    pub balance_minor: String,
    /// Human-readable USD string derived from `balance_minor`.
    pub balance_usd: String,
    pub usd_scale: u32,
    pub currency: &'static str,
}

pub async fn get_my_balance(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let minor = db::get_balance_minor(&conn, user_id)
        .map_err(|e| ApiError::InternalError(format!("balance: {e}")))?;
    Ok(HttpResponse::Ok().json(BalanceResponse {
        balance_minor: minor.to_string(),
        balance_usd: money::minor_to_string(minor),
        usd_scale: money::USD_MINOR_EXP,
        currency: "USD",
    }))
}

#[derive(Serialize)]
pub struct LedgerItem {
    pub id: i64,
    pub created_at: i64,
    pub kind: String,
    pub amount_minor: String,
    pub amount_usd: String,
    pub balance_after_minor: String,
    pub balance_after_usd: String,
    pub request_id: Option<String>,
    pub model: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub external_ref: Option<String>,
}

#[derive(Serialize)]
pub struct LedgerResponse {
    pub data: Vec<LedgerItem>,
}

pub async fn get_my_ledger(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<LedgerQuery>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);
    let kind = query.kind.as_deref();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let rows = db::list_billing_ledger(&conn, user_id, kind, limit, offset)
        .map_err(|e| ApiError::InternalError(format!("ledger: {e}")))?;
    let data = rows
        .into_iter()
        .map(|r| LedgerItem {
            id: r.id,
            created_at: r.created_at,
            kind: r.kind,
            amount_minor: r.amount_minor.to_string(),
            amount_usd: money::minor_to_string(r.amount_minor),
            balance_after_minor: r.balance_after_minor.to_string(),
            balance_after_usd: money::minor_to_string(r.balance_after_minor),
            request_id: r.request_id,
            model: r.model,
            prompt_tokens: r.prompt_tokens,
            completion_tokens: r.completion_tokens,
            external_ref: r.external_ref,
        })
        .collect();
    Ok(HttpResponse::Ok().json(LedgerResponse { data }))
}

#[derive(serde::Deserialize)]
pub struct LedgerQuery {
    pub kind: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn bearer_token_value(req: &HttpRequest) -> Option<&str> {
    let raw = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    const PREFIX: &str = "Bearer ";
    if !raw.starts_with(PREFIX) {
        return None;
    }
    Some(raw[PREFIX.len()..].trim())
}

fn admin_password_eq(expected: &str, presented: &str) -> bool {
    let e = expected.as_bytes();
    let p = presented.as_bytes();
    if e.len() != p.len() {
        return false;
    }
    bool::from(e.ct_eq(p))
}

#[derive(serde::Deserialize)]
pub struct AdminDepositBody {
    pub username: String,
    pub amount_usd: f64,
}

#[derive(Serialize)]
pub struct AdminDepositResponse {
    pub user_id: i64,
    pub username: String,
    pub balance_minor: String,
    pub balance_usd: String,
    pub usd_scale: u32,
    pub currency: &'static str,
}

/// Manual top-up for a user by console username. Requires `billing.admin_deposit_enabled`,
/// non-empty `billing.admin_deposit_password`, and matching `Authorization: Bearer`.
pub async fn post_admin_deposit(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<AdminDepositBody>,
) -> Result<HttpResponse, ApiError> {
    let cfg = &state.cfg.billing;
    if !cfg.admin_deposit_enabled || cfg.admin_deposit_password.is_empty() {
        return Err(ApiError::NotFound("Not found".into()));
    }
    let presented = bearer_token_value(&req)
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid Authorization header".into()))?;
    if !admin_password_eq(cfg.admin_deposit_password.trim(), presented) {
        return Err(ApiError::Unauthorized("Invalid credentials".into()));
    }

    let username = body.username.trim();
    if username.is_empty() {
        return Err(ApiError::BadRequest("username is required".into()));
    }

    let amount = body.amount_usd;
    if !amount.is_finite() || amount <= 0.0 {
        return Err(ApiError::BadRequest(
            "amount_usd must be a positive finite number".into(),
        ));
    }
    let amount_dec = Decimal::from_f64(amount).ok_or_else(|| {
        ApiError::BadRequest("amount_usd could not be converted to decimal".into())
    })?;
    let add_minor = money::usd_to_minor(amount_dec);
    let min_minor = money::cents_to_minor(cfg.min_deposit_cents);
    if add_minor < min_minor {
        return Err(ApiError::BadRequest(format!(
            "amount_usd must be at least {} USD",
            money::minor_to_string(min_minor)
        )));
    }

    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let user_id = match db::find_user_id(&conn, username) {
        Ok(id) => id,
        Err(RusqliteError::QueryReturnedNoRows) => {
            return Err(ApiError::NotFound("user not found".into()));
        }
        Err(e) => return Err(ApiError::InternalError(format!("user lookup: {e}"))),
    };

    let now = crate::audit::now_unix_secs();
    let external = format!("admin_deposit:{now}");
    let after = db::billing_deposit(&conn, user_id, add_minor, now, Some(&external))
        .map_err(|e| ApiError::InternalError(format!("deposit: {e}")))?;
    Ok(HttpResponse::Ok().json(AdminDepositResponse {
        user_id,
        username: username.to_string(),
        balance_minor: after.to_string(),
        balance_usd: money::minor_to_string(after),
        usd_scale: money::USD_MINOR_EXP,
        currency: "USD",
    }))
}
