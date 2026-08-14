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
            | "xapikey"
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

fn is_text_key_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')
}

fn line_value_end(text: &str, start: usize) -> usize {
    text[start..]
        .find(['\r', '\n'])
        .map_or(text.len(), |relative| start + relative)
}

fn delimited_value_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    if matches!(bytes.get(start), Some(b'"' | b'\'')) {
        let quote = bytes[start];
        let mut index = start + 1;
        let mut escaped = false;
        while index < bytes.len() {
            let byte = bytes[index];
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == quote {
                return index;
            }
            index += 1;
        }
        return text.len();
    }

    let mut index = if text[start..].starts_with(REDACTION_MARKER) {
        start + REDACTION_MARKER.len()
    } else {
        start
    };
    while index < bytes.len()
        && !bytes[index].is_ascii_whitespace()
        && !matches!(
            bytes[index],
            b';' | b',' | b'&' | b'#' | b'"' | b'\'' | b'}' | b']'
        )
    {
        index += 1;
    }
    index
}

fn is_redaction_marker_text(value: &str) -> bool {
    value == REDACTION_MARKER
        || ['"', '\''].iter().any(|quote| {
            value
                .strip_prefix(*quote)
                .and_then(|inner| inner.strip_suffix(*quote))
                == Some(REDACTION_MARKER)
        })
}

fn sanitize_credentials_in_text(text: &str) -> (String, u32) {
    let bytes = text.as_bytes();
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut index = 0;
    let mut redacted_values = 0u32;

    while index < bytes.len() {
        if !bytes[index].is_ascii_alphabetic() || (index > 0 && is_text_key_byte(bytes[index - 1]))
        {
            index += 1;
            continue;
        }

        let key_start = index;
        while index < bytes.len() && is_text_key_byte(bytes[index]) {
            index += 1;
        }
        let key_end = index;
        let key = &text[key_start..key_end];
        if !is_sensitive_key(key) {
            continue;
        }

        let mut separator = key_end;
        if key_start > 0
            && matches!(bytes[key_start - 1], b'"' | b'\'')
            && bytes.get(key_end) == Some(&bytes[key_start - 1])
        {
            separator += 1;
        }
        while matches!(bytes.get(separator), Some(b' ' | b'\t')) {
            separator += 1;
        }
        if !matches!(bytes.get(separator), Some(b':' | b'=')) {
            continue;
        }

        let mut value_start = separator + 1;
        while matches!(bytes.get(value_start), Some(b' ' | b'\t')) {
            value_start += 1;
        }
        let quoted_content_start = if matches!(bytes.get(value_start), Some(b'"' | b'\'')) {
            value_start + 1
        } else {
            value_start
        };
        if quoted_content_start >= bytes.len() {
            index = quoted_content_start;
            continue;
        }

        let line_wide = matches!(
            normalized_key(key).as_str(),
            "authorization" | "proxyauthorization"
        );
        let replacement_start = if line_wide {
            value_start
        } else {
            quoted_content_start
        };
        let mut value_end = if line_wide {
            line_value_end(text, value_start)
        } else {
            delimited_value_end(text, value_start)
        };
        if replacement_start >= value_end {
            continue;
        }

        while value_end > replacement_start && bytes[value_end - 1].is_ascii_whitespace() {
            value_end -= 1;
        }
        if is_redaction_marker_text(&text[replacement_start..value_end]) {
            index = value_end;
            continue;
        }
        output.push_str(&text[cursor..replacement_start]);
        output.push_str(REDACTION_MARKER);
        cursor = value_end;
        index = value_end;
        redacted_values = redacted_values.saturating_add(1);
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
            // Some exporters encode tool output as a JSON string. Decode only
            // object/array strings and keep ordinary text byte-for-byte stable.
            if let Ok(mut decoded) = serde_json::from_str::<Value>(text)
                && matches!(decoded, Value::Object(_) | Value::Array(_))
            {
                let before = *redacted_values;
                redact_value(&mut decoded, path, configured_paths, redacted_values);
                if *redacted_values > before {
                    if let Ok(encoded) = serde_json::to_string(&decoded) {
                        *text = encoded;
                    }
                    return;
                }
            }

            let (sanitized_urls, url_count) = sanitize_urls_in_text(text);
            let (sanitized, credential_count) = sanitize_credentials_in_text(&sanitized_urls);
            let count = url_count.saturating_add(credential_count);
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
