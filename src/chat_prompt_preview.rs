//! Extract a short preview of the latest user message from OpenAI-style chat completion JSON.

use serde_json::Value;

const MAX_PREVIEW_CHARS: usize = 1024;

/// Returns trimmed text from the last `messages[]` entry with `role: user`, or `None`.
pub fn extract_chat_user_prompt_preview(body: &[u8]) -> Option<String> {
    let root: Value = serde_json::from_slice(body).ok()?;
    let messages = root.get("messages")?.as_array()?;
    for msg in messages.iter().rev() {
        let role = msg.get("role").and_then(|v| v.as_str())?;
        if role != "user" {
            continue;
        }
        let Some(content) = msg.get("content") else {
            continue;
        };
        let Some(text) = message_content_as_text(content) else {
            continue;
        };
        let t = text.trim();
        if t.is_empty() {
            continue;
        }
        return Some(truncate_preview(t, MAX_PREVIEW_CHARS));
    }
    None
}

fn message_content_as_text(content: &Value) -> Option<String> {
    match content {
        Value::String(s) => Some(s.clone()),
        Value::Array(parts) => {
            let mut out = String::new();
            for p in parts {
                let ty = p.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if ty == "text" {
                    if let Some(t) = p.get("text").and_then(|v| v.as_str()) {
                        if !out.is_empty() {
                            out.push(' ');
                        }
                        out.push_str(t);
                    }
                } else if !ty.is_empty() {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push('[');
                    out.push_str(ty);
                    out.push(']');
                }
            }
            if out.is_empty() {
                None
            } else {
                Some(out)
            }
        }
        Value::Null => None,
        _ => None,
    }
}

fn truncate_preview(s: &str, max_chars: usize) -> String {
    let n = s.chars().count();
    if n <= max_chars {
        return s.to_string();
    }
    s.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_user_string_content() {
        let j = br#"{"model":"m","messages":[
            {"role":"system","content":"sys"},
            {"role":"user","content":"first"},
            {"role":"assistant","content":"ok"},
            {"role":"user","content":"  final answer  "}
        ]}"#;
        assert_eq!(
            extract_chat_user_prompt_preview(j).as_deref(),
            Some("final answer")
        );
    }

    #[test]
    fn multimodal_parts() {
        let j = br#"{"model":"m","messages":[{"role":"user","content":[
            {"type":"text","text":"Hello"},
            {"type":"image_url","image_url":{"url":"http://x"}}
        ]}]}"#;
        let p = extract_chat_user_prompt_preview(j).expect("preview");
        assert!(p.starts_with("Hello"));
        assert!(p.contains("[image_url]"));
    }

    #[test]
    fn invalid_json_returns_none() {
        assert!(extract_chat_user_prompt_preview(b"not json").is_none());
    }
}
