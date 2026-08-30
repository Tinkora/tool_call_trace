use serde_json::Value;
use std::io::Write;
use std::process::{Command, Output, Stdio};

const PYDANTIC_AI: &str =
    include_str!("../../tool_call_trace_core/tests/fixtures/pydantic_ai_logfire_spans.json");

fn run_cli(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tool-call-trace"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn checks_generic_json_from_stdin() {
    let input = r#"[
      {"id":"call_1","name":"search","input":{"query":"hello"},"start_time_ms":0,"end_time_ms":10,"status":"success"}
    ]"#;
    let output = run_cli(&["check", "--format", "generic", "-"], input);

    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["valid"], true);
    assert_eq!(report["redacted_values"], 0);
    assert_eq!(report["log"]["calls"][0]["name"], "search");
    assert!(String::from_utf8_lossy(&output.stderr).contains("valid: 1 tool call"));
}

#[test]
fn auto_detects_a_pinned_sdk_fixture_from_a_file() {
    let path = std::env::temp_dir().join(format!(
        "tool-call-trace-pydantic-{}-{}.json",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    std::fs::write(&path, PYDANTIC_AI).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_tool-call-trace"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();

    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["log"]["calls"][0]["name"], "add_numbers");
}

#[test]
fn redacts_before_writing_json_or_diagnostics() {
    let input = r#"[
      {
        "id":"call_searchable_01",
        "name":"fetch",
        "input":{
          "url":"https://user:URL_SECRET_9x@example.test/mcp?token=QUERY_SECRET_9x#fragment",
          "authorization":"Bearer AUTH_SECRET_9x",
          "customer":{"email":"EMAIL_SECRET_9x"}
        },
        "start_time_ms":0,
        "end_time_ms":10,
        "status":"success"
      }
    ]"#;
    let output = run_cli(
        &[
            "check",
            "--redact",
            "--redact-path",
            "/input/customer/email",
            "-",
        ],
        input,
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(output.status.success());
    for secret in [
        "URL_SECRET_9x",
        "QUERY_SECRET_9x",
        "AUTH_SECRET_9x",
        "EMAIL_SECRET_9x",
    ] {
        assert!(!combined.contains(secret), "secret remained: {secret}");
    }
    assert!(combined.contains("call_searchable_01"));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["redacted_values"], 3);
}

#[test]
fn invalid_input_fails_without_echoing_the_document() {
    let output = run_cli(&["check", "-"], "{\"secret\":\"PARSE_SECRET_9x\"");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("PARSE_ERROR"));
    assert!(!stderr.contains("PARSE_SECRET_9x"));
    assert!(output.stdout.is_empty());
}

#[test]
fn redaction_mode_hides_values_from_contract_errors() {
    let input = r#"[
      {"id":"call_1","name":"search","input":{},"start_time_ms":0,"end_time_ms":1,"status":"STATUS_ERROR_SECRET_9x"}
    ]"#;
    let output = run_cli(&["check", "--format", "generic", "--redact", "-"], input);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("INVALID_FORMAT"));
    assert!(!stderr.contains("STATUS_ERROR_SECRET_9x"));
    assert!(output.stdout.is_empty());
}

#[test]
fn rejects_paths_when_redaction_is_not_enabled() {
    let output = Command::new(env!("CARGO_BIN_EXE_tool-call-trace"))
        .args(["check", "--redact-path", "/input/token", "-"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--redact-path requires --redact"));
}

#[test]
fn reports_retry_loop_findings_without_input_or_error_values() {
    let input = r#"[
      {"id":"1","name":"Search","input":{"token":"SECRET_INPUT"},"error":"SECRET_ERROR","start_time_ms":0,"end_time_ms":10,"status":"error"},
      {"id":"2","name":"search","input":{"token":"SECRET_INPUT"},"error":"SECRET_ERROR","start_time_ms":10,"end_time_ms":20,"status":"error"},
      {"id":"3","name":"SEARCH","input":{"token":"SECRET_INPUT"},"error":"SECRET_ERROR","start_time_ms":20,"end_time_ms":30,"status":"error"}
    ]"#;

    let output = run_cli(&["check", "--format", "generic", "-"], input);

    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["retry_loop_findings"][0]["kind"],
        "consecutive_failures"
    );
    assert_eq!(report["retry_loop_findings"][0]["tool_name"], "search");
    assert_eq!(report["retry_loop_findings"][0]["call_count"], 3);
    let finding = report["retry_loop_findings"][0].to_string();
    assert!(!finding.contains("SECRET_INPUT"));
    assert!(!finding.contains("SECRET_ERROR"));
}

#[test]
fn redaction_does_not_merge_distinct_inputs_into_a_retry_loop() {
    let input = r#"[
      {"id":"1","name":"fetch","input":{"token":"SECRET_A"},"start_time_ms":0,"end_time_ms":10,"status":"error"},
      {"id":"2","name":"fetch","input":{"token":"SECRET_B"},"start_time_ms":10,"end_time_ms":20,"status":"error"},
      {"id":"3","name":"fetch","input":{"token":"SECRET_C"},"start_time_ms":20,"end_time_ms":30,"status":"error"}
    ]"#;

    let output = run_cli(&["check", "--format", "generic", "--redact", "-"], input);

    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["retry_loop_findings"], serde_json::json!([]));
    let rendered = String::from_utf8_lossy(&output.stdout);
    assert!(!rendered.contains("SECRET_A"));
    assert!(!rendered.contains("SECRET_B"));
    assert!(!rendered.contains("SECRET_C"));
}
