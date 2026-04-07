//! Team and member HTTP API (`/api/v1/teams`, invitations).

use actix_web::{web, HttpRequest, HttpResponse};
use bcrypt::{hash, DEFAULT_COST};
use rand::Rng;
use rusqlite::ErrorCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::db;
use crate::{errors::ApiError, session_auth, AppState};

fn auth_user_id(req: &HttpRequest, state: &web::Data<AppState>) -> Result<i64, ApiError> {
    Ok(session_auth::resolve_console_session(req, state)?.user_id)
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn invite_token_hash(raw: &str) -> String {
    hex::encode(Sha256::digest(raw.as_bytes()))
}

fn slug_valid(s: &str) -> bool {
    let s = s.trim();
    (2..=48).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[derive(Serialize)]
pub struct TeamOut {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub created_by_user_id: i64,
    pub created_at: i64,
    /// Caller’s role in this team.
    pub role: String,
}

#[derive(Deserialize)]
pub struct CreateTeamBody {
    pub name: String,
    pub slug: String,
}

#[derive(Serialize)]
pub struct CreateTeamResponse {
    pub team: TeamOut,
}

fn team_out(row: db::TeamRow, role: String) -> TeamOut {
    TeamOut {
        id: row.id,
        name: row.name,
        slug: row.slug,
        created_by_user_id: row.created_by_user_id,
        created_at: row.created_at,
        role,
    }
}

pub async fn list_my_teams(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let rows = db::list_teams_for_user(&conn, user_id)
        .map_err(|_| ApiError::InternalError("failed to list teams".into()))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let role = db::team_member_role(&conn, row.id, user_id)
            .map_err(|_| ApiError::InternalError("database error".into()))?
            .unwrap_or_else(|| "member".into());
        out.push(team_out(row, role));
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({ "data": out })))
}

pub async fn create_team(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateTeamBody>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let name = body.name.trim();
    let slug = body.slug.trim().to_lowercase();
    if name.is_empty() || name.len() > 128 {
        return Err(ApiError::BadRequest("name must be 1–128 characters".into()));
    }
    if !slug_valid(&slug) {
        return Err(ApiError::BadRequest(
            "slug must be 2–48 chars: lowercase letters, digits, hyphens".into(),
        ));
    }
    let created_at = now_secs();
    let mut conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let tx = conn
        .transaction()
        .map_err(|_| ApiError::InternalError("database error".into()))?;
    let tid = db::insert_team(&tx, name, &slug, user_id, created_at).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            ApiError::Conflict("team slug already exists".into())
        } else {
            ApiError::InternalError(format!("failed to create team: {e}"))
        }
    })?;
    db::add_team_member(&tx, tid, user_id, "owner", created_at)
        .map_err(|e| ApiError::InternalError(format!("failed to add owner: {e}")))?;
    tx.commit()
        .map_err(|_| ApiError::InternalError("failed to commit team".into()))?;
    let row = db::get_team_by_id(&conn, tid)
        .map_err(|_| ApiError::InternalError("failed to load team".into()))?;
    Ok(HttpResponse::Created().json(CreateTeamResponse {
        team: team_out(row, "owner".into()),
    }))
}

pub async fn get_team(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let team_id = path.into_inner();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    if !db::user_is_team_member(&conn, team_id, user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
    {
        return Err(ApiError::NotFound("team not found".into()));
    }
    let row = db::get_team_by_id(&conn, team_id)
        .map_err(|_| ApiError::NotFound("team not found".into()))?;
    let role = db::team_member_role(&conn, team_id, user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
        .unwrap_or_else(|| "member".into());
    Ok(HttpResponse::Ok().json(team_out(row, role)))
}

#[derive(Deserialize)]
pub struct PatchTeamBody {
    pub name: Option<String>,
    pub slug: Option<String>,
}

pub async fn patch_team(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    body: web::Json<PatchTeamBody>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let team_id = path.into_inner();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    if !db::is_team_admin_or_owner(&conn, team_id, user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
    {
        return Err(ApiError::Forbidden("team admin or owner required".into()));
    }
    let name = body.name.as_ref().map(|s| s.trim().to_string());
    let slug = body
        .slug
        .as_ref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());
    if let Some(ref n) = name {
        if n.is_empty() || n.len() > 128 {
            return Err(ApiError::BadRequest("name must be 1–128 characters".into()));
        }
    }
    if let Some(ref s) = slug {
        if !slug_valid(s) {
            return Err(ApiError::BadRequest(
                "slug must be 2–48 chars: lowercase letters, digits, hyphens".into(),
            ));
        }
    }
    db::update_team_name_slug(&conn, team_id, name.as_deref(), slug.as_deref()).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            ApiError::Conflict("team slug already exists".into())
        } else {
            ApiError::InternalError(format!("update team: {e}"))
        }
    })?;
    let row = db::get_team_by_id(&conn, team_id)
        .map_err(|_| ApiError::NotFound("team not found".into()))?;
    let role = db::team_member_role(&conn, team_id, user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
        .unwrap_or_else(|| "member".into());
    Ok(HttpResponse::Ok().json(team_out(row, role)))
}

pub async fn delete_team(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let team_id = path.into_inner();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let role = db::team_member_role(&conn, team_id, user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?;
    if !matches!(role.as_deref(), Some("owner")) {
        return Err(ApiError::Forbidden(
            "only team owner can delete team".into(),
        ));
    }
    let n = db::delete_team(&conn, team_id)
        .map_err(|e| ApiError::InternalError(format!("delete team: {e}")))?;
    if n == 0 {
        return Err(ApiError::NotFound("team not found".into()));
    }
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Serialize)]
pub struct MemberOut {
    pub user_id: i64,
    pub username: String,
    pub role: String,
    pub joined_at: i64,
}

pub async fn list_team_members(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let team_id = path.into_inner();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    if !db::user_is_team_member(&conn, team_id, user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
    {
        return Err(ApiError::NotFound("team not found".into()));
    }
    let rows = db::list_team_members(&conn, team_id)
        .map_err(|_| ApiError::InternalError("failed to list members".into()))?;
    let data: Vec<MemberOut> = rows
        .into_iter()
        .map(|r| MemberOut {
            user_id: r.user_id,
            username: r.username,
            role: r.role,
            joined_at: r.joined_at,
        })
        .collect();
    Ok(HttpResponse::Ok().json(serde_json::json!({ "data": data })))
}

#[derive(Deserialize)]
pub struct InviteBody {
    pub invitee_username: String,
    pub role: String,
}

#[derive(Serialize)]
pub struct InviteCreatedResponse {
    pub id: i64,
    pub team_id: i64,
    pub invitee_username: String,
    pub role: String,
    pub expires_at: i64,
    /// One-time secret; only shown on creation.
    pub token: String,
}

pub async fn create_invitation(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    body: web::Json<InviteBody>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let team_id = path.into_inner();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    if !db::is_team_admin_or_owner(&conn, team_id, user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
    {
        return Err(ApiError::Forbidden("team admin or owner required".into()));
    }
    let invitee = body.invitee_username.trim();
    if invitee.is_empty() {
        return Err(ApiError::BadRequest("invitee_username required".into()));
    }
    let role = body.role.trim().to_lowercase();
    if !matches!(role.as_str(), "member" | "admin") {
        return Err(ApiError::BadRequest(
            "role must be \"member\" or \"admin\"".into(),
        ));
    }
    let _invitee_uid = db::find_user_id(&conn, invitee)
        .map_err(|_| ApiError::NotFound("user not found for invitation".into()))?;
    if db::user_is_team_member(&conn, team_id, _invitee_uid)
        .map_err(|_| ApiError::InternalError("database error".into()))?
    {
        return Err(ApiError::Conflict("user is already a team member".into()));
    }
    let raw_token = format!("mg_inv_{}", uuid::Uuid::new_v4());
    let token_hash = invite_token_hash(&raw_token);
    let created_at = now_secs();
    let expires_at = created_at + 7 * 86400;
    let id = db::insert_team_invitation(
        &conn,
        team_id,
        invitee,
        &role,
        &token_hash,
        user_id,
        created_at,
        expires_at,
    )
    .map_err(|e| ApiError::InternalError(format!("invitation: {e}")))?;
    Ok(HttpResponse::Created().json(InviteCreatedResponse {
        id,
        team_id,
        invitee_username: invitee.to_string(),
        role,
        expires_at,
        token: raw_token,
    }))
}

fn provisioning_api_key() -> String {
    let mut rng = rand::thread_rng();
    let random_part: String = (0..32)
        .map(|_| format!("{:x}", rng.gen::<u8>() % 16))
        .collect();
    format!("sk-or-v1-{}", random_part)
}

#[derive(Deserialize)]
pub struct RegisterMemberOnBehalfBody {
    pub username: String,
    pub password: String,
    pub role: String,
}

#[derive(Serialize)]
pub struct RegisterMemberOnBehalfResponse {
    pub user_id: i64,
    pub username: String,
}

/// Team admin/owner creates a new console account (no platform invite code) and adds them to the team.
pub async fn register_member_on_behalf(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    body: web::Json<RegisterMemberOnBehalfBody>,
) -> Result<HttpResponse, ApiError> {
    let caller_id = auth_user_id(&req, &state)?;
    let team_id = path.into_inner();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    if !db::is_team_admin_or_owner(&conn, team_id, caller_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
    {
        return Err(ApiError::Forbidden("team admin or owner required".into()));
    }

    let u = body.username.trim();
    if u.is_empty() {
        return Err(ApiError::BadRequest("username is required".into()));
    }
    if u.len() > 64 {
        return Err(ApiError::BadRequest(
            "username must be at most 64 characters".into(),
        ));
    }
    let password = body.password.trim();
    if password.len() < 8 {
        return Err(ApiError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }
    if db::find_user_id(&conn, u).is_ok() {
        return Err(ApiError::Conflict("username already exists".into()));
    }
    let role = body.role.trim().to_lowercase();
    if !matches!(role.as_str(), "member" | "admin") {
        return Err(ApiError::BadRequest(
            "role must be \"member\" or \"admin\"".into(),
        ));
    }

    let password_hash = hash(password, DEFAULT_COST)
        .map_err(|_| ApiError::InternalError("password hashing failed".into()))?;
    let api_key = provisioning_api_key();
    let created_at = now_secs();
    let mut conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let tx = conn
        .transaction()
        .map_err(|_| ApiError::InternalError("database error".into()))?;
    let user_id = match db::insert_user_with_password(&tx, u, &password_hash, created_at) {
        Ok(id) => id,
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == ErrorCode::ConstraintViolation => {
            return Err(ApiError::Conflict("username already exists".into()));
        }
        Err(_) => {
            return Err(ApiError::InternalError("failed to create user".into()));
        }
    };
    if let Err(e) = db::insert_api_key_for_user(&tx, user_id, &api_key, created_at) {
        return Err(ApiError::InternalError(format!("api key: {e}")));
    }
    if let Err(e) = db::add_team_member(&tx, team_id, user_id, &role, created_at) {
        return Err(ApiError::InternalError(format!("add team member: {e}")));
    }
    tx.commit()
        .map_err(|_| ApiError::InternalError("commit failed".into()))?;

    Ok(
        HttpResponse::Created().json(RegisterMemberOnBehalfResponse {
            user_id,
            username: u.to_string(),
        }),
    )
}

#[derive(Deserialize)]
pub struct AcceptInviteBody {
    pub token: String,
}

pub async fn accept_invitation(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<AcceptInviteBody>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let raw = body.token.trim();
    if raw.is_empty() {
        return Err(ApiError::BadRequest("token required".into()));
    }
    let mut conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let hash = invite_token_hash(raw);
    let inv = db::find_pending_invitation_by_hash(&conn, &hash)
        .map_err(|_| ApiError::BadRequest("invalid or expired invitation".into()))?;
    let username: String = conn
        .query_row(
            "SELECT username FROM users WHERE id = ?1",
            rusqlite::params![user_id],
            |row| row.get(0),
        )
        .map_err(|_| ApiError::InternalError("user lookup failed".into()))?;
    if username != inv.invitee_username {
        return Err(ApiError::Forbidden(
            "invitation was issued to a different username".into(),
        ));
    }
    let at = now_secs();
    let tx = conn
        .transaction()
        .map_err(|_| ApiError::InternalError("database error".into()))?;
    if !db::user_is_team_member(&tx, inv.team_id, user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
    {
        db::add_team_member(&tx, inv.team_id, user_id, &inv.role, at)
            .map_err(|e| ApiError::InternalError(format!("add member: {e}")))?;
    }
    let n = db::mark_invitation_accepted(&tx, inv.id, at)
        .map_err(|_| ApiError::InternalError("mark invitation".into()))?;
    tx.commit()
        .map_err(|_| ApiError::InternalError("commit".into()))?;
    if n == 0 {
        return Err(ApiError::BadRequest("invitation already used".into()));
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "team_id": inv.team_id,
        "role": inv.role,
    })))
}

pub async fn delete_invitation(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let (team_id, invitation_id) = path.into_inner();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    if !db::is_team_admin_or_owner(&conn, team_id, user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
    {
        return Err(ApiError::Forbidden("team admin or owner required".into()));
    }
    let n = db::delete_team_invitation(&conn, invitation_id, team_id)
        .map_err(|e| ApiError::InternalError(format!("delete invitation: {e}")))?;
    if n == 0 {
        return Err(ApiError::NotFound("invitation not found".into()));
    }
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize)]
pub struct PatchMemberBody {
    pub role: String,
}

pub async fn patch_team_member(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    body: web::Json<PatchMemberBody>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let (team_id, target_user_id) = path.into_inner();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    if !db::is_team_admin_or_owner(&conn, team_id, user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
    {
        return Err(ApiError::Forbidden("team admin or owner required".into()));
    }
    let new_role = body.role.trim().to_lowercase();
    if !matches!(new_role.as_str(), "member" | "admin") {
        return Err(ApiError::BadRequest(
            "role must be \"member\" or \"admin\"".into(),
        ));
    }
    let current = db::team_member_role(&conn, team_id, target_user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
        .ok_or_else(|| ApiError::NotFound("member not found".into()))?;
    if current == "owner" {
        return Err(ApiError::Forbidden("cannot change owner role here".into()));
    }
    let n = db::set_team_member_role(&conn, team_id, target_user_id, &new_role)
        .map_err(|e| ApiError::InternalError(format!("update role: {e}")))?;
    if n == 0 {
        return Err(ApiError::NotFound("member not found".into()));
    }
    Ok(HttpResponse::Ok().finish())
}

pub async fn remove_team_member(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let (team_id, target_user_id) = path.into_inner();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    if !db::is_team_admin_or_owner(&conn, team_id, user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
    {
        return Err(ApiError::Forbidden("team admin or owner required".into()));
    }
    let current = db::team_member_role(&conn, team_id, target_user_id)
        .map_err(|_| ApiError::InternalError("database error".into()))?
        .ok_or_else(|| ApiError::NotFound("member not found".into()))?;
    if current == "owner" {
        let owners = db::count_team_owners(&conn, team_id)
            .map_err(|_| ApiError::InternalError("database error".into()))?;
        if owners <= 1 {
            return Err(ApiError::BadRequest("cannot remove the only owner".into()));
        }
    }
    let n = db::remove_team_member(&conn, team_id, target_user_id)
        .map_err(|e| ApiError::InternalError(format!("remove member: {e}")))?;
    if n == 0 {
        return Err(ApiError::NotFound("member not found".into()));
    }
    Ok(HttpResponse::NoContent().finish())
}
