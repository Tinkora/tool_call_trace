use crate::error::CoreError;
use crate::parse::ToolCallLog;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use url::Url;

pub const REDACTION_MARKER: &str = "[REDACTED]";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RedactionConfig {
    /// Exact JSON Pointer paths relative to each normalized call.
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RedactionOutcome {
    pub log: ToolCallLog,
    pub redacted_values: u32,
}

fn normalized_key(key: &str) -> String {
    key.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_sensitive_key(key: &str) -> bool {
    matches!(
        normalized_key(key).as_str(),
        "authorization"
            | "proxyauthorization"
            | "apikey"
            | "accesskey"
            | "accesstoken"
            | "authtoken"
            | "bearertoken"
            | "clientsecret"
            | "privatekey"
            | "refreshtoken"
            | "secret"
            | "secretkey"
            | "sessiontoken"
            | "password"
            | "passwd"
            | "token"
    )
}

fn is_redacted(value: &Value) -> bool {
    value.as_str() == Some(REDACTION_MARKER)
}

fn replace_value(value: &mut Value, redacted_values: &mut u32) {
    if !is_redacted(value) {
        *value = Value::String(REDACTION_MARKER.into());
        *redacted_values = redacted_values.saturating_add(1);
    }
}

fn pointer_segment(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn validate_pointer(path: &str) -> Result<(), CoreError> {
    let in_scope = path == "/input"
        || path.starts_with("/input/")
        || path == "/output"
        || path.starts_with("/output/")
        || path == "/error";
    if !in_scope {
        return Err(CoreError::InvalidFormat(
            "redaction paths must target /input, /output, or /error".into(),
        ));
    }

    let bytes = path.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'~' {
            if index + 1 >= bytes.len() || !matches!(bytes[index + 1], b'0' | b'1') {
                return Err(CoreError::InvalidFormat(
                    "redaction paths must use valid JSON Pointer escapes".into(),
                ));
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(())
}

fn sanitize_url(url: &str) -> Option<String> {
    let mut parsed = Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    let had_sensitive_component = !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some();
    if !had_sensitive_component {
        return None;
    }

    parsed.set_username("").ok()?;
    parsed.set_password(None).ok()?;
    parsed.set_query(None);
    parsed.set_fragment(None);
    Some(parsed.into())
}

fn find_http_start(text: &str, offset: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = offset;
    while index < bytes.len() {
        let remaining = &bytes[index..];
        let is_http = remaining.len() >= 7 && remaining[..7].eq_ignore_ascii_case(b"http://");
        let is_https = remaining.len() >= 8 && remaining[..8].eq_ignore_ascii_case(b"https://");
        if is_http || is_https {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn candidate_end(text: &str, start: usize) -> usize {
    for (relative, character) in text[start..].char_indices() {
        if character.is_whitespace() || matches!(character, '\'' | '"' | '<' | '>') {
            return start + relative;
        }
    }
    text.len()
}

fn sanitize_urls_in_text(text: &str) -> (String, u32) {
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut redacted_values = 0u32;

    while let Some(start) = find_http_start(text, cursor) {
        output.push_str(&text[cursor..start]);
        let end = candidate_end(text, start);
        let candidate = &text[start..end];
        let trimmed = candidate.trim_end_matches(['.', ',', ';', ')', ']', '}']);
        let suffix = &candidate[trimmed.len()..];
        if let Some(sanitized) = sanitize_url(trimmed) {
            output.push_str(&sanitized);
            output.push_str(suffix);
            redacted_values = redacted_values.saturating_add(1);
        } else {
            output.push_str(candidate);
        }
        cursor = end;
    }
    output.push_str(&text[cursor..]);
    (output, redacted_values)
}

fn redact_value(
    value: &mut Value,
    path: &str,
    configured_paths: &HashSet<&str>,
    redacted_values: &mut u32,
) {
    if configured_paths.contains(path) {
        replace_value(value, redacted_values);
        return;
    }

    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if is_sensitive_key(key) {
                    replace_value(child, redacted_values);
                } else {
                    let child_path = format!("{path}/{}", pointer_segment(key));
                    redact_value(child, &child_path, configured_paths, redacted_values);
                }
            }
        }
        Value::Array(array) => {
            for (index, child) in array.iter_mut().enumerate() {
                let child_path = format!("{path}/{index}");
                redact_value(child, &child_path, configured_paths, redacted_values);
            }
        }
        Value::String(text) if text != REDACTION_MARKER => {
            let (sanitized, count) = sanitize_urls_in_text(text);
            if count > 0 {
                *text = sanitized;
                *redacted_values = redacted_values.saturating_add(count);
            }
        }
        _ => {}
    }
}

/// Redacts a normalized log while preserving trace and tool-call identifiers.
pub fn redact_log(
    log: &ToolCallLog,
    config: &RedactionConfig,
) -> Result<RedactionOutcome, CoreError> {
    for path in &config.paths {
        validate_pointer(path)?;
    }
    let configured_paths: HashSet<&str> = config.paths.iter().map(String::as_str).collect();
    let mut log = log.clone();
    let mut redacted_values = 0u32;

    for call in &mut log.calls {
        redact_value(
            &mut call.input,
            "/input",
            &configured_paths,
            &mut redacted_values,
        );
        if let Some(output) = &mut call.output {
            redact_value(output, "/output", &configured_paths, &mut redacted_values);
        }
        if let Some(error) = &mut call.error {
            let mut value = Value::String(std::mem::take(error));
            redact_value(
                &mut value,
                "/error",
                &configured_paths,
                &mut redacted_values,
            );
            *error = value.as_str().unwrap_or(REDACTION_MARKER).to_owned();
        }
    }

    Ok(RedactionOutcome {
        log,
        redacted_values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_urls_without_sensitive_components_unchanged() {
        let input = "See https://example.com/public and continue";
        assert_eq!(sanitize_urls_in_text(input), (input.into(), 0));
    }

    #[test]
    fn redacts_urls_next_to_terminal_punctuation() {
        let input = "Failed (https://user:pass@example.com/a?token=x#part).";
        assert_eq!(
            sanitize_urls_in_text(input),
            ("Failed (https://example.com/a).".into(), 1)
        );
    }
}
