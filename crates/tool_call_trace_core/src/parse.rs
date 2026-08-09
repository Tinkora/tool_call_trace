use crate::error::CoreError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_INPUT_BYTES: usize = 5 * 1024 * 1024;
pub const MAX_TOOL_CALLS: usize = 2_000;

/// The status of an individual tool call.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CallStatus {
    /// The tool call completed successfully.
    Success,
    /// The tool call returned an error.
    Error,
    /// The tool call was cancelled before completion.
    Cancelled,
    /// The tool call is still pending (not yet completed).
    Pending,
}

/// Represents a single tool call within a trace log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call (e.g. "call_abc123", "toolu_xxx").
    pub id: String,
    /// Human-readable tool/function name (e.g. "search", "read_file").
    pub name: String,
    /// The input arguments passed to the tool.
    pub input: serde_json::Value,
    /// The output returned by the tool, if available.
    pub output: Option<serde_json::Value>,
    /// Error message, if the call failed.
    pub error: Option<String>,
    /// Start time in milliseconds relative to trace start.
    pub start_time_ms: u64,
    /// End time in milliseconds relative to trace start.
    pub end_time_ms: u64,
    /// Duration of the call in milliseconds.
    pub duration_ms: u64,
    /// Execution status.
    pub status: CallStatus,
}

/// A parsed tool-call log containing all calls and summary metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCallLog {
    /// Unique identifier for this trace (generated or extracted from input).
    pub trace_id: String,
    /// All tool calls in this trace, sorted by start_time_ms.
    pub calls: Vec<ToolCall>,
    /// Total wall-clock time of the trace in milliseconds.
    pub total_time_ms: u64,
    /// Total number of tool calls.
    pub total_calls: u32,
    /// Number of calls that resulted in error.
    pub error_count: u32,
}

fn normalize_timestamps(calls: &mut [ToolCall], trace_start_ms: u64) {
    for call in calls {
        call.start_time_ms = call.start_time_ms.saturating_sub(trace_start_ms);
        call.end_time_ms = call.end_time_ms.saturating_sub(trace_start_ms);
    }
}

fn validate_input_size(json: &str) -> Result<(), CoreError> {
    if json.len() > MAX_INPUT_BYTES {
        return Err(CoreError::InvalidFormat(format!(
            "input exceeds the {MAX_INPUT_BYTES}-byte limit"
        )));
    }
    Ok(())
}

fn validate_call_count(count: usize) -> Result<(), CoreError> {
    if count > MAX_TOOL_CALLS {
        return Err(CoreError::InvalidFormat(format!(
            "tool-call count exceeds the {MAX_TOOL_CALLS}-call limit"
        )));
    }
    Ok(())
}

fn deserialize_input<T: DeserializeOwned>(json: &str) -> Result<T, CoreError> {
    serde_json::from_str(json).map_err(|error| {
        if error.is_data() {
            CoreError::InvalidFormat(error.to_string())
        } else {
            CoreError::ParseError(error.to_string())
        }
    })
}

// ---------------------------------------------------------------------------
// OpenAI format support
// ---------------------------------------------------------------------------

/// A minimal representation of an OpenAI run step for parsing.
#[derive(Debug, Deserialize)]
struct OpenAIStepList {
    data: Vec<OpenAIStep>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStep {
    id: String,
    step_details: OpenAIStepDetails,
    created_at: u64,
    completed_at: Option<u64>,
    failed_at: Option<u64>,
    cancelled_at: Option<u64>,
    expired_at: Option<u64>,
    status: OpenAIStatus,
}

#[derive(Debug, Deserialize)]
struct OpenAIStepDetails {
    #[serde(rename = "type")]
    step_type: String,
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAIFunction,
}

#[derive(Debug, Deserialize)]
struct OpenAIFunction {
    name: String,
    arguments: String,
    output: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OpenAIStatus {
    Completed,
    Failed,
    Cancelled,
    Expired,
    InProgress,
}

/// Parse a tool-call log in OpenAI run-steps format.
///
/// Expects a JSON object with `"object": "list"` and a `"data"` array
/// of run step objects, each containing `step_details.type == "tool_calls"`.
pub fn parse_openai_format(json: &str) -> Result<ToolCallLog, CoreError> {
    validate_input_size(json)?;
    let list: OpenAIStepList = deserialize_input(json)?;

    let mut calls: Vec<ToolCall> = Vec::new();
    let mut min_time = u64::MAX;
    let mut max_time = 0u64;
    let mut errors = 0u32;
    let mut seen_ids = HashSet::new();

    for step in &list.data {
        if step.step_details.step_type != "tool_calls" {
            continue;
        }
        if step.id.trim().is_empty() {
            return Err(CoreError::InvalidFormat("step id must not be empty".into()));
        }
        let tcs =
            step.step_details.tool_calls.as_ref().ok_or_else(|| {
                CoreError::InvalidFormat("step_details.tool_calls is null".into())
            })?;

        let required_end = |value: Option<u64>, field: &str| {
            value.ok_or_else(|| {
                CoreError::InvalidFormat(format!(
                    "{field} is required for {} step {}",
                    match step.status {
                        OpenAIStatus::Completed => "completed",
                        OpenAIStatus::Failed => "failed",
                        OpenAIStatus::Cancelled => "cancelled",
                        OpenAIStatus::Expired => "expired",
                        OpenAIStatus::InProgress => "in_progress",
                    },
                    step.id
                ))
            })
        };
        let (end_seconds, end_field, status) = match step.status {
            OpenAIStatus::Completed => (
                required_end(step.completed_at, "completed_at")?,
                "completed_at",
                CallStatus::Success,
            ),
            OpenAIStatus::Failed => (
                required_end(step.failed_at, "failed_at")?,
                "failed_at",
                CallStatus::Error,
            ),
            OpenAIStatus::Cancelled => (
                required_end(step.cancelled_at, "cancelled_at")?,
                "cancelled_at",
                CallStatus::Cancelled,
            ),
            OpenAIStatus::Expired => (
                required_end(step.expired_at, "expired_at")?,
                "expired_at",
                CallStatus::Cancelled,
            ),
            OpenAIStatus::InProgress => (step.created_at, "created_at", CallStatus::Pending),
        };
        if end_seconds < step.created_at {
            return Err(CoreError::InvalidFormat(format!(
                "{end_field} precedes created_at for step {}",
                step.id,
            )));
        }
        let start_ms = step.created_at.checked_mul(1_000).ok_or_else(|| {
            CoreError::InvalidFormat(format!(
                "created_at overflows milliseconds for step {}",
                step.id
            ))
        })?;
        let end_ms = end_seconds.checked_mul(1_000).ok_or_else(|| {
            CoreError::InvalidFormat(format!(
                "{end_field} overflows milliseconds for step {}",
                step.id,
            ))
        })?;

        for tc in tcs {
            validate_call_count(calls.len() + 1)?;
            if tc.id.trim().is_empty() {
                return Err(CoreError::InvalidFormat(
                    "tool-call id must not be empty".into(),
                ));
            }
            if !seen_ids.insert(tc.id.clone()) {
                return Err(CoreError::InvalidFormat(format!(
                    "duplicate tool-call id: {}",
                    tc.id
                )));
            }
            if tc.call_type != "function" {
                return Err(CoreError::InvalidFormat(format!(
                    "unsupported tool-call type for {}: {}",
                    tc.id, tc.call_type
                )));
            }
            if tc.function.name.trim().is_empty() {
                return Err(CoreError::InvalidFormat(format!(
                    "function name must not be empty for {}",
                    tc.id
                )));
            }
            let input: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).map_err(|error| {
                    CoreError::InvalidFormat(format!(
                        "invalid function arguments for {}: {error}",
                        tc.id
                    ))
                })?;

            if status == CallStatus::Error {
                errors += 1;
            }
            let duration = end_ms - start_ms;

            if start_ms < min_time {
                min_time = start_ms;
            }
            if end_ms > max_time {
                max_time = end_ms;
            }

            calls.push(ToolCall {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                input,
                output: tc.function.output.clone().map(serde_json::Value::String),
                error: if status == CallStatus::Error {
                    Some(format!("Step {} failed", step.id))
                } else {
                    None
                },
                start_time_ms: start_ms,
                end_time_ms: end_ms,
                duration_ms: duration,
                status,
            });
        }
    }

    if calls.is_empty() {
        return Err(CoreError::EmptyLog);
    }

    // Sort by start time
    calls.sort_by_key(|c| c.start_time_ms);

    let total_time = if min_time != u64::MAX {
        max_time.saturating_sub(min_time)
    } else {
        0
    };
    normalize_timestamps(&mut calls, min_time);

    Ok(ToolCallLog {
        trace_id: format!(
            "openai-{}",
            &calls.first().map(|c| c.id.as_str()).unwrap_or("unknown")
        ),
        total_calls: calls.len() as u32,
        total_time_ms: total_time,
        error_count: errors,
        calls,
    })
}

// ---------------------------------------------------------------------------
// Generic array format support
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GenericToolCall {
    id: String,
    name: String,
    input: serde_json::Value,
    output: Option<serde_json::Value>,
    error: Option<String>,
    start_time_ms: u64,
    end_time_ms: u64,
    status: GenericCallStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GenericCallStatus {
    #[serde(alias = "completed")]
    Success,
    #[serde(alias = "failed")]
    Error,
    Cancelled,
    #[serde(alias = "in_progress")]
    Pending,
}

/// Parse a tool-call log in generic flat-array format.
///
/// Expects a JSON array of objects, each with: id, name, input, optional
/// output, error, start_time_ms, end_time_ms, status.
pub fn parse_generic_array(json: &str) -> Result<ToolCallLog, CoreError> {
    validate_input_size(json)?;
    let raw_calls: Vec<GenericToolCall> = deserialize_input(json)?;

    if raw_calls.is_empty() {
        return Err(CoreError::EmptyLog);
    }
    validate_call_count(raw_calls.len())?;

    let mut calls: Vec<ToolCall> = Vec::new();
    let mut min_time = u64::MAX;
    let mut max_time = 0u64;
    let mut errors = 0u32;
    let mut seen_ids = HashSet::with_capacity(raw_calls.len());

    for raw in raw_calls {
        if raw.id.trim().is_empty() {
            return Err(CoreError::InvalidFormat("id must not be empty".into()));
        }
        if raw.name.trim().is_empty() {
            return Err(CoreError::InvalidFormat("name must not be empty".into()));
        }
        if !seen_ids.insert(raw.id.clone()) {
            return Err(CoreError::InvalidFormat(format!(
                "duplicate tool-call id: {}",
                raw.id
            )));
        }
        if raw.end_time_ms < raw.start_time_ms {
            return Err(CoreError::InvalidFormat(format!(
                "end_time_ms precedes start_time_ms for {}",
                raw.id
            )));
        }

        let status = match raw.status {
            GenericCallStatus::Success => CallStatus::Success,
            GenericCallStatus::Error => {
                errors += 1;
                CallStatus::Error
            }
            GenericCallStatus::Cancelled => CallStatus::Cancelled,
            GenericCallStatus::Pending => CallStatus::Pending,
        };
        let duration_ms = raw.end_time_ms - raw.start_time_ms;

        if raw.start_time_ms < min_time {
            min_time = raw.start_time_ms;
        }
        if raw.end_time_ms > max_time {
            max_time = raw.end_time_ms;
        }

        calls.push(ToolCall {
            id: raw.id,
            name: raw.name,
            input: raw.input,
            output: raw.output,
            error: raw.error,
            start_time_ms: raw.start_time_ms,
            end_time_ms: raw.end_time_ms,
            duration_ms,
            status,
        });
    }

    // Sort by start time
    calls.sort_by_key(|c| c.start_time_ms);

    let total_time = if min_time != u64::MAX {
        max_time.saturating_sub(min_time)
    } else {
        0
    };
    normalize_timestamps(&mut calls, min_time);

    Ok(ToolCallLog {
        trace_id: format!(
            "generic-{}",
            &calls.first().map(|c| c.id.as_str()).unwrap_or("unknown")
        ),
        total_calls: calls.len() as u32,
        total_time_ms: total_time,
        error_count: errors,
        calls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_generic_array() {
        let json = r#"[
            {"id":"1","name":"search","input":{"q":"hello"},"output":{"r":[]},"start_time_ms":0,"end_time_ms":100,"status":"success"},
            {"id":"2","name":"read_file","input":{"path":"x"},"output":null,"start_time_ms":50,"end_time_ms":200,"status":"error","error":"not found"},
            {"id":"3","name":"search","input":{"q":"bye"},"start_time_ms":200,"end_time_ms":350,"status":"pending"}
        ]"#;
        let log = parse_generic_array(json).unwrap();
        assert_eq!(log.total_calls, 3);
        assert_eq!(log.error_count, 1);
        assert_eq!(log.total_time_ms, 350);
        assert_eq!(log.calls[0].name, "search");
        assert_eq!(log.calls[1].status, CallStatus::Error);
        assert_eq!(log.calls[2].status, CallStatus::Pending);
    }

    #[test]
    fn generic_timestamps_are_relative_to_trace_start() {
        let json = r#"[
            {"id":"1","name":"search","input":{},"start_time_ms":1000,"end_time_ms":1100,"status":"success"},
            {"id":"2","name":"read_file","input":{},"start_time_ms":1050,"end_time_ms":1350,"status":"success"}
        ]"#;

        let log = parse_generic_array(json).unwrap();

        assert_eq!(log.total_time_ms, 350);
        assert_eq!(log.calls[0].start_time_ms, 0);
        assert_eq!(log.calls[0].end_time_ms, 100);
        assert_eq!(log.calls[1].start_time_ms, 50);
        assert_eq!(log.calls[1].end_time_ms, 350);
    }

    #[test]
    fn test_parse_empty_array() {
        let result = parse_generic_array("[]");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CoreError::EmptyLog);
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_generic_array("not json");
        assert!(matches!(result.unwrap_err(), CoreError::ParseError(_)));
    }

    #[test]
    fn generic_rejects_missing_required_fields() {
        let result = parse_generic_array(
            r#"[{"id":"1","input":{},"start_time_ms":0,"end_time_ms":1,"status":"success"}]"#,
        );

        assert!(matches!(result, Err(CoreError::InvalidFormat(_))));
    }

    #[test]
    fn generic_rejects_unknown_status() {
        let result = parse_generic_array(
            r#"[{"id":"1","name":"search","input":{},"start_time_ms":0,"end_time_ms":1,"status":"done-ish"}]"#,
        );

        assert!(matches!(result, Err(CoreError::InvalidFormat(_))));
    }

    #[test]
    fn generic_rejects_end_before_start() {
        let result = parse_generic_array(
            r#"[{"id":"1","name":"search","input":{},"start_time_ms":2,"end_time_ms":1,"status":"success"}]"#,
        );

        assert!(matches!(result, Err(CoreError::InvalidFormat(_))));
    }

    #[test]
    fn generic_rejects_duplicate_ids() {
        let result = parse_generic_array(
            r#"[
                {"id":"same","name":"search","input":{},"start_time_ms":0,"end_time_ms":1,"status":"success"},
                {"id":"same","name":"read","input":{},"start_time_ms":1,"end_time_ms":2,"status":"success"}
            ]"#,
        );

        assert!(matches!(result, Err(CoreError::InvalidFormat(_))));
    }

    #[test]
    fn generic_rejects_oversized_input_before_parsing() {
        let input = " ".repeat(5 * 1024 * 1024 + 1);

        let result = parse_generic_array(&input);

        assert!(matches!(result, Err(CoreError::InvalidFormat(_))));
    }

    #[test]
    fn generic_rejects_more_calls_than_the_ui_can_render() {
        let calls: Vec<_> = (0..=2_000)
            .map(|index| {
                serde_json::json!({
                    "id": format!("call-{index}"),
                    "name": "search",
                    "input": {},
                    "start_time_ms": index,
                    "end_time_ms": index + 1,
                    "status": "success"
                })
            })
            .collect();
        let input = serde_json::to_string(&calls).unwrap();

        let result = parse_generic_array(&input);

        assert!(matches!(result, Err(CoreError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_openai_format() {
        let json = r#"{
            "object": "list",
            "data": [{
                "id": "step_1",
                "step_details": {
                    "type": "tool_calls",
                    "tool_calls": [{
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "search",
                            "arguments": "{\"query\":\"hello\"}",
                            "output": "{\"matches\":3}"
                        }
                    }]
                },
                "created_at": 1715000000,
                "completed_at": 1715000005,
                "status": "completed"
            }]
        }"#;
        let log = parse_openai_format(json).unwrap();
        assert_eq!(log.total_calls, 1);
        assert_eq!(log.calls[0].name, "search");
        assert_eq!(log.calls[0].status, CallStatus::Success);
        assert_eq!(log.calls[0].start_time_ms, 0);
        assert_eq!(log.calls[0].end_time_ms, 5000);
        assert_eq!(log.calls[0].duration_ms, 5000);
        // input should be parsed from the arguments JSON string
        assert_eq!(log.calls[0].input["query"], "hello");
        assert_eq!(
            log.calls[0].output,
            Some(serde_json::Value::String("{\"matches\":3}".into()))
        );
    }

    #[test]
    fn openai_rejects_unknown_status() {
        let json = r#"{
            "data": [{
                "id": "step_1",
                "step_details": {
                    "type": "tool_calls",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {"name": "search", "arguments": "{}"}
                    }]
                },
                "created_at": 1,
                "completed_at": 2,
                "status": "mostly_done"
            }]
        }"#;

        assert!(matches!(
            parse_openai_format(json),
            Err(CoreError::InvalidFormat(_))
        ));
    }

    #[test]
    fn openai_rejects_invalid_function_arguments() {
        let json = r#"{
            "data": [{
                "id": "step_1",
                "step_details": {
                    "type": "tool_calls",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {"name": "search", "arguments": "not json"}
                    }]
                },
                "created_at": 1,
                "completed_at": 2,
                "status": "completed"
            }]
        }"#;

        assert!(matches!(
            parse_openai_format(json),
            Err(CoreError::InvalidFormat(_))
        ));
    }

    #[test]
    fn openai_rejects_terminal_steps_without_an_end_timestamp() {
        let json = r#"{
            "data": [{
                "id": "step_1",
                "step_details": {
                    "type": "tool_calls",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {"name": "search", "arguments": "{}"}
                    }]
                },
                "created_at": 1,
                "status": "failed"
            }]
        }"#;

        assert!(matches!(
            parse_openai_format(json),
            Err(CoreError::InvalidFormat(_))
        ));
    }

    #[test]
    fn openai_uses_the_end_timestamp_for_each_terminal_status() {
        for (status, end_field, expected_status, expected_errors) in [
            ("completed", "completed_at", CallStatus::Success, 0),
            ("failed", "failed_at", CallStatus::Error, 1),
            ("cancelled", "cancelled_at", CallStatus::Cancelled, 0),
            ("expired", "expired_at", CallStatus::Cancelled, 0),
        ] {
            let mut document = serde_json::json!({
                "data": [{
                    "id": "step_1",
                    "step_details": {
                        "type": "tool_calls",
                        "tool_calls": [{
                            "id": "call_1",
                            "type": "function",
                            "function": {"name": "search", "arguments": "{}"}
                        }]
                    },
                    "created_at": 1,
                    "status": status
                }]
            });
            document["data"][0][end_field] = serde_json::json!(3);

            let log = parse_openai_format(&document.to_string()).unwrap();

            assert_eq!(log.calls[0].status, expected_status, "status: {status}");
            assert_eq!(log.calls[0].duration_ms, 2_000, "status: {status}");
            assert_eq!(log.error_count, expected_errors, "status: {status}");
        }
    }

    #[test]
    fn openai_rejects_duplicate_call_ids() {
        let json = r#"{
            "data": [{
                "id": "step_1",
                "step_details": {
                    "type": "tool_calls",
                    "tool_calls": [
                        {"id": "same", "type": "function", "function": {"name": "search", "arguments": "{}"}},
                        {"id": "same", "type": "function", "function": {"name": "read", "arguments": "{}"}}
                    ]
                },
                "created_at": 1,
                "completed_at": 2,
                "status": "completed"
            }]
        }"#;

        assert!(matches!(
            parse_openai_format(json),
            Err(CoreError::InvalidFormat(_))
        ));
    }

    #[test]
    fn openai_rejects_reversed_or_overflowing_timestamps() {
        let reversed = r#"{
                "data": [{
                    "id": "step_1",
                    "step_details": {
                        "type": "tool_calls",
                        "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "search", "arguments": "{}"}}]
                    },
                    "created_at": 2,
                    "completed_at": 1,
                    "status": "completed"
                }]
            }"#
            .to_string();
        let overflowing = reversed
            .replace(
                "\"created_at\": 2",
                &format!("\"created_at\": {}", u64::MAX),
            )
            .replace(
                "\"completed_at\": 1",
                &format!("\"completed_at\": {}", u64::MAX),
            );

        assert!(matches!(
            parse_openai_format(&reversed),
            Err(CoreError::InvalidFormat(_))
        ));
        assert!(matches!(
            parse_openai_format(&overflowing),
            Err(CoreError::InvalidFormat(_))
        ));
    }
}
