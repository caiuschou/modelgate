use serde_json::Value;

pub fn build_chat_completions_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');

    if base.ends_with("/chat/completions") {
        return base.to_string();
    }

    // OpenAI: .../v1  →  .../v1/chat/completions
    // 火山方舟 / 部分兼容网关: .../api/v3 或 .../api/coding/v3  →  .../v3/chat/completions（不要再拼 /v1）
    if base.ends_with("/v1") || base.ends_with("/v3") {
        return format!("{base}/chat/completions");
    }
    if base.ends_with("/api") {
        return format!("{base}/v1/chat/completions");
    }

    format!("{base}/v1/chat/completions")
}

/// Resolves `GET …/models` against the same upstream base as chat completions.
///
/// Implementation derives the path from [`build_chat_completions_url`] so the models endpoint
/// always stays the sibling of whatever chat URL the gateway would use — avoiding drift if
/// `base_url` shapes are extended for chat only.
pub fn build_models_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');

    if base.ends_with("/models") {
        return base.to_string();
    }

    let chat_url = build_chat_completions_url(base_url);
    if let Some(prefix) = chat_url.strip_suffix("/chat/completions") {
        return format!("{prefix}/models");
    }

    format!("{base}/v1/models")
}

pub fn is_stream_request(body: &[u8]) -> bool {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| value.get("stream").and_then(|s| s.as_bool()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_chat_completions_url_returns_existing_chat_url() {
        assert_eq!(
            build_chat_completions_url("https://api.example.com/v1/chat/completions"),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn build_chat_completions_url_appends_v1_chat_for_v1_base() {
        assert_eq!(
            build_chat_completions_url("https://api.example.com/v1"),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn build_chat_completions_url_appends_v1_chat_for_api_base() {
        assert_eq!(
            build_chat_completions_url("https://api.example.com/api"),
            "https://api.example.com/api/v1/chat/completions"
        );
    }

    #[test]
    fn build_chat_completions_url_defaults_to_v1_chat() {
        assert_eq!(
            build_chat_completions_url("https://api.example.com"),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn build_models_url_for_v1_base() {
        assert_eq!(
            build_models_url("https://api.example.com/v1"),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn build_models_url_for_api_base() {
        assert_eq!(
            build_models_url("https://api.example.com/api"),
            "https://api.example.com/api/v1/models"
        );
    }

    #[test]
    fn build_models_url_returns_existing_models_url() {
        assert_eq!(
            build_models_url("https://api.example.com/v1/models"),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn build_models_url_strips_chat_completions_suffix() {
        assert_eq!(
            build_models_url("https://api.example.com/v1/chat/completions"),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn build_models_url_defaults_to_v1_models() {
        assert_eq!(
            build_models_url("https://api.example.com"),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn build_models_url_matches_chat_sibling_for_openrouter_style_base() {
        let base = "https://openrouter.ai/api/v1";
        assert_eq!(
            build_chat_completions_url(base),
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(
            build_models_url(base),
            "https://openrouter.ai/api/v1/models"
        );
    }

    /// 火山方舟 Coding Plan：base 为 `.../api/coding/v3`，对话路径为 `.../v3/chat/completions`（不是 `.../v3/v1/...`）。
    #[test]
    fn build_chat_and_models_for_volcengine_ark_coding_v3() {
        let base = "https://ark.cn-beijing.volces.com/api/coding/v3";
        assert_eq!(
            build_chat_completions_url(base),
            "https://ark.cn-beijing.volces.com/api/coding/v3/chat/completions"
        );
        assert_eq!(
            build_models_url(base),
            "https://ark.cn-beijing.volces.com/api/coding/v3/models"
        );
    }

    #[test]
    fn build_chat_for_volcengine_ark_standard_api_v3() {
        let base = "https://ark.cn-beijing.volces.com/api/v3";
        assert_eq!(
            build_chat_completions_url(base),
            "https://ark.cn-beijing.volces.com/api/v3/chat/completions"
        );
    }

    #[test]
    fn is_stream_request_detects_true_and_false() {
        assert!(is_stream_request(b"{\"stream\": true}"));
        assert!(!is_stream_request(b"{\"stream\": false}"));
        assert!(!is_stream_request(b"{}"));
        assert!(!is_stream_request(b"not json"));
    }
}
