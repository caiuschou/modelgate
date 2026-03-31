use serde_json::Value;

pub fn build_chat_completions_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');

    if base.ends_with("/chat/completions") {
        return base.to_string();
    }

    if base.ends_with("/v1") {
        return format!("{base}/chat/completions");
    }
    if base.ends_with("/api") {
        return format!("{base}/v1/chat/completions");
    }

    format!("{base}/v1/chat/completions")
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
        assert_eq!(build_chat_completions_url("https://api.example.com/v1/chat/completions"), "https://api.example.com/v1/chat/completions");
    }

    #[test]
    fn build_chat_completions_url_appends_v1_chat_for_v1_base() {
        assert_eq!(build_chat_completions_url("https://api.example.com/v1"), "https://api.example.com/v1/chat/completions");
    }

    #[test]
    fn build_chat_completions_url_appends_v1_chat_for_api_base() {
        assert_eq!(build_chat_completions_url("https://api.example.com/api"), "https://api.example.com/api/v1/chat/completions");
    }

    #[test]
    fn build_chat_completions_url_defaults_to_v1_chat() {
        assert_eq!(build_chat_completions_url("https://api.example.com"), "https://api.example.com/v1/chat/completions");
    }

    #[test]
    fn is_stream_request_detects_true_and_false() {
        assert!(is_stream_request(b"{\"stream\": true}"));
        assert!(!is_stream_request(b"{\"stream\": false}"));
        assert!(!is_stream_request(b"{}"));
        assert!(!is_stream_request(b"not json"));
    }
}
