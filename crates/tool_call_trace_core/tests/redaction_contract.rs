use tool_call_trace_core::{RedactionConfig, ToolCallLog, parse_generic_array, redact_log};

const URL_PASSWORD: &str = "URL_PASSWORD_SENTINEL_9x";
const QUERY_TOKEN: &str = "QUERY_TOKEN_SENTINEL_9x";
const AUTH_TOKEN: &str = "AUTH_TOKEN_SENTINEL_9x";
const API_KEY: &str = "API_KEY_SENTINEL_9x";
const X_API_KEY: &str = "X_API_KEY_SENTINEL_9x";
const CUSTOM_VALUE: &str = "CUSTOM_VALUE_SENTINEL_9x";

fn sensitive_log() -> ToolCallLog {
    parse_generic_array(&format!(
        r#"[
          {{
            "id": "call_searchable_01",
            "name": "fetch",
            "input": {{
              "url": "https://client:{URL_PASSWORD}@api.example.test/mcp?api_key={QUERY_TOKEN}#section",
              "headers": {{"Authorization": "Bearer {AUTH_TOKEN}"}},
              "nested": [{{"api-key": "{API_KEY}"}}],
              "customer": {{"email": "{CUSTOM_VALUE}"}},
              "request_id": "request_searchable_01"
            }},
            "output": [{{"session": "{CUSTOM_VALUE}"}}],
            "error": "Failed https://client:{URL_PASSWORD}@api.example.test/mcp?api_key={QUERY_TOKEN}#section",
            "start_time_ms": 10,
            "end_time_ms": 20,
            "status": "error"
          }}
        ]"#
    ))
    .unwrap()
}

#[test]
fn default_rules_remove_sensitive_fields_and_url_components() {
    let outcome = redact_log(&sensitive_log(), &RedactionConfig::default()).unwrap();
    let serialized = serde_json::to_string(&outcome).unwrap();

    for secret in [URL_PASSWORD, QUERY_TOKEN, AUTH_TOKEN, API_KEY] {
        assert!(!serialized.contains(secret), "secret remained: {secret}");
    }
    assert!(serialized.contains("https://api.example.test/mcp"));
    assert!(serialized.contains("request_searchable_01"));
    assert_eq!(outcome.log.trace_id, "generic-call_searchable_01");
    assert_eq!(outcome.log.calls[0].id, "call_searchable_01");
    assert_eq!(outcome.redacted_values, 4);
}

#[test]
fn default_rules_recognize_the_standard_x_api_key_header() {
    let log = parse_generic_array(&format!(
        r#"[
          {{
            "id":"call_1",
            "name":"fetch",
            "input":{{"headers":{{"X-API-Key":"{X_API_KEY}"}}}},
            "start_time_ms":0,
            "end_time_ms":1,
            "status":"success"
          }}
        ]"#
    ))
    .unwrap();

    let outcome = redact_log(&log, &RedactionConfig::default()).unwrap();
    let serialized = serde_json::to_string(&outcome).unwrap();

    assert!(!serialized.contains(X_API_KEY));
    assert_eq!(
        outcome.log.calls[0].input["headers"]["X-API-Key"],
        "[REDACTED]"
    );
    assert_eq!(outcome.redacted_values, 1);
}

#[test]
fn configured_json_pointers_redact_exact_input_and_output_values() {
    let config = RedactionConfig {
        paths: vec!["/input/customer/email".into(), "/output/0/session".into()],
    };
    let outcome = redact_log(&sensitive_log(), &config).unwrap();
    let serialized = serde_json::to_string(&outcome).unwrap();

    assert!(!serialized.contains(CUSTOM_VALUE));
    assert_eq!(outcome.redacted_values, 6);
    assert_eq!(
        outcome.log.calls[0].input["customer"]["email"],
        "[REDACTED]"
    );
    assert_eq!(
        outcome.log.calls[0].output.as_ref().unwrap()[0]["session"],
        "[REDACTED]"
    );
}

#[test]
fn malformed_or_out_of_scope_paths_fail_before_transforming_the_log() {
    for path in [
        "input/token",
        "/trace_id",
        "/input/bad~2escape",
        "/error/detail",
    ] {
        let error = redact_log(
            &sensitive_log(),
            &RedactionConfig {
                paths: vec![path.into()],
            },
        )
        .unwrap_err();

        assert_eq!(error.code(), "INVALID_FORMAT");
        assert!(!error.to_string().contains(AUTH_TOKEN));
        assert!(!error.to_string().contains(URL_PASSWORD));
    }
}

#[test]
fn disabled_custom_paths_leave_non_sensitive_values_unchanged() {
    let outcome = redact_log(&sensitive_log(), &RedactionConfig::default()).unwrap();

    assert_eq!(
        outcome.log.calls[0].input["customer"]["email"],
        CUSTOM_VALUE
    );
    assert_eq!(
        outcome.log.calls[0].output.as_ref().unwrap()[0]["session"],
        CUSTOM_VALUE
    );
}

#[test]
fn redaction_is_idempotent() {
    let first = redact_log(&sensitive_log(), &RedactionConfig::default()).unwrap();
    let second = redact_log(&first.log, &RedactionConfig::default()).unwrap();

    assert_eq!(second.redacted_values, 0);
    assert_eq!(
        serde_json::to_value(first.log).unwrap(),
        serde_json::to_value(second.log).unwrap()
    );
}
