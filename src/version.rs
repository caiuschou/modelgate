//! Crate version and git identity (set at compile time).

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_SHA: &str = env!("MODELGATE_GIT_SHA");

pub fn full_version_line() -> String {
    format!("modelgate {VERSION} ({GIT_SHA})")
}

pub fn version_json() -> serde_json::Value {
    serde_json::json!({
        "name": "modelgate",
        "version": VERSION,
        "git_sha": GIT_SHA,
    })
}
