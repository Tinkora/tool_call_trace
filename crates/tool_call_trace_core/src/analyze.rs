use crate::parse::{ToolCall, ToolCallLog};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_RETRY_FAILURE_THRESHOLD: usize = 3;

/// A retry-related pattern found without exposing tool input or error values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryLoopFinding {
    pub kind: RetryFindingKind,
    pub tool_name: String,
    pub call_count: u32,
    pub call_ids: Vec<String>,
    pub message: String,
}

/// Stable machine-readable categories for retry-related findings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryFindingKind {
    ConsecutiveFailures,
    RecoveredRetryLoop,
    OverlappingDuplicate,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CanonicalCallKey {
    tool_name: String,
    input: String,
}

fn canonical_call_key(call: &ToolCall) -> CanonicalCallKey {
    CanonicalCallKey {
        tool_name: call.name.trim().to_lowercase(),
        input: canonical_json(&call.input),
    }
}

fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by_key(|(key, _)| *key);
            let body = entries
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap_or_default(),
                        canonical_json(value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{body}}}")
        }
        serde_json::Value::Array(values) => {
            let body = values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{body}]")
        }
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// Detect retry loops and concurrent duplicate executions using a fixed threshold of three errors.
pub fn find_retry_loop_findings(log: &ToolCallLog) -> Vec<RetryLoopFinding> {
    let mut findings = Vec::new();
    let calls = &log.calls;
    let mut index = 0;

    while index < calls.len() {
        if calls[index].status != crate::parse::CallStatus::Error {
            index += 1;
            continue;
        }
        let key = canonical_call_key(&calls[index]);
        let start = index;
        while index < calls.len()
            && calls[index].status == crate::parse::CallStatus::Error
            && canonical_call_key(&calls[index]) == key
        {
            index += 1;
        }
        let failure_count = index - start;
        if failure_count < DEFAULT_RETRY_FAILURE_THRESHOLD {
            continue;
        }

        let recovered = index < calls.len()
            && calls[index].status == crate::parse::CallStatus::Success
            && canonical_call_key(&calls[index]) == key;
        let end = index + usize::from(recovered);
        let kind = if recovered {
            RetryFindingKind::RecoveredRetryLoop
        } else {
            RetryFindingKind::ConsecutiveFailures
        };
        findings.push(RetryLoopFinding {
            kind,
            tool_name: key.tool_name,
            call_count: (end - start) as u32,
            call_ids: calls[start..end]
                .iter()
                .map(|call| call.id.clone())
                .collect(),
            message: if recovered {
                format!("Repeated failures recovered after {failure_count} attempts")
            } else {
                format!("Detected {failure_count} consecutive failed attempts")
            },
        });
        if recovered {
            index += 1;
        }
    }

    let mut active: HashMap<CanonicalCallKey, Vec<&ToolCall>> = HashMap::new();
    for call in calls {
        let key = canonical_call_key(call);
        let overlapping: Vec<_> = active
            .entry(key.clone())
            .or_default()
            .iter()
            .copied()
            .filter(|previous| previous.end_time_ms > call.start_time_ms)
            .collect();
        for previous in overlapping {
            findings.push(RetryLoopFinding {
                kind: RetryFindingKind::OverlappingDuplicate,
                tool_name: key.tool_name.clone(),
                call_count: 2,
                call_ids: vec![previous.id.clone(), call.id.clone()],
                message: "Identical tool calls overlap in time".into(),
            });
        }
        let same_key = active.entry(key).or_default();
        same_key.retain(|previous| previous.end_time_ms > call.start_time_ms);
        same_key.push(call);
    }

    findings
}

/// Statistical analysis of a tool-call trace.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceAnalysis {
    /// Average duration across all calls in milliseconds.
    pub avg_duration_ms: f64,
    /// Minimum call duration in milliseconds.
    pub min_duration_ms: u64,
    /// Maximum call duration in milliseconds.
    pub max_duration_ms: u64,
    /// Name of the uniquely most frequently called tool, or empty on a tie.
    pub most_called_tool: String,
    /// Error rate as a fraction (0.0 = no errors, 1.0 = all errors).
    pub error_rate: f64,
    /// Total bytes of input JSON across all calls.
    pub total_input_bytes: u64,
    /// Total bytes of output JSON across all calls.
    pub total_output_bytes: u64,
}

/// Compute aggregate statistics over a parsed tool-call log.
pub fn analyze(log: &ToolCallLog) -> TraceAnalysis {
    let n = log.calls.len() as f64;
    if n == 0.0 {
        return TraceAnalysis {
            avg_duration_ms: 0.0,
            min_duration_ms: 0,
            max_duration_ms: 0,
            most_called_tool: String::new(),
            error_rate: 0.0,
            total_input_bytes: 0,
            total_output_bytes: 0,
        };
    }

    let total_duration: u128 = log
        .calls
        .iter()
        .map(|call| u128::from(call.duration_ms))
        .sum();
    let avg = total_duration as f64 / n;

    let min = log.calls.iter().map(|c| c.duration_ms).min().unwrap_or(0);
    let max = log.calls.iter().map(|c| c.duration_ms).max().unwrap_or(0);

    // Most called tool
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for call in &log.calls {
        *counts.entry(&call.name).or_insert(0) += 1;
    }
    let max_count = counts.values().copied().max().unwrap_or(0);
    let mut most_called_candidates = counts.into_iter().filter(|(_, count)| *count == max_count);
    let most_called = match (most_called_candidates.next(), most_called_candidates.next()) {
        (Some((name, _)), None) => name.to_string(),
        _ => String::new(),
    };

    let err_rate = if log.total_calls > 0 {
        log.error_count as f64 / log.total_calls as f64
    } else {
        0.0
    };

    // Compute byte sizes
    let total_input: u64 = log
        .calls
        .iter()
        .map(|c| serde_json::to_string(&c.input).unwrap_or_default().len() as u64)
        .fold(0, u64::saturating_add);
    let total_output: u64 = log
        .calls
        .iter()
        .map(|c| {
            c.output
                .as_ref()
                .map(|o| serde_json::to_string(o).unwrap_or_default().len() as u64)
                .unwrap_or(0)
        })
        .fold(0, u64::saturating_add);

    TraceAnalysis {
        avg_duration_ms: avg,
        min_duration_ms: min,
        max_duration_ms: max,
        most_called_tool: most_called,
        error_rate: err_rate,
        total_input_bytes: total_input,
        total_output_bytes: total_output,
    }
}

/// Detect repeated identical calls (same tool name + same input).
///
/// Returns a list of (tool_name, count) for any tool+input combination
/// that appears more than once. This helps identify unnecessary retries
/// or duplicate calls.
pub fn find_duplicate_calls(log: &ToolCallLog) -> Vec<(String, u32)> {
    let mut seen: HashMap<(&str, String), u32> = HashMap::new();

    for call in &log.calls {
        let input_str = serde_json::to_string(&call.input).unwrap_or_default();
        *seen.entry((&call.name, input_str)).or_insert(0) += 1;
    }

    let mut duplicate_groups: Vec<_> = seen.into_iter().filter(|(_, count)| *count > 1).collect();
    duplicate_groups.sort_by(
        |((name_a, input_a), count_a), ((name_b, input_b), count_b)| {
            count_b
                .cmp(count_a)
                .then_with(|| name_a.cmp(name_b))
                .then_with(|| input_a.cmp(input_b))
        },
    );

    duplicate_groups
        .into_iter()
        .map(|((name, _), count)| (format!("{name} (×{count})"), count))
        .collect()
}

/// Find tool calls that exceed a given duration threshold.
///
/// Returns references to all ToolCalls whose duration_ms > threshold_ms.
pub fn find_slow_calls(log: &ToolCallLog, threshold_ms: u64) -> Vec<&ToolCall> {
    log.calls
        .iter()
        .filter(|c| c.duration_ms > threshold_ms)
        .collect()
}

/// Full analysis including duplicates and slow calls, serializable for WASM.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullAnalysis {
    pub stats: TraceAnalysis,
    pub duplicate_calls: Vec<(String, u32)>,
    pub slow_calls: Vec<SlowCallInfo>,
    pub total_calls: u32,
    pub error_count: u32,
    pub total_time_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlowCallInfo {
    pub id: String,
    pub name: String,
    pub duration_ms: u64,
    pub status: String,
}

/// Run the complete analysis suite on a log.
pub fn full_analyze(log: &ToolCallLog, slow_threshold_ms: Option<u64>) -> FullAnalysis {
    let threshold = slow_threshold_ms.unwrap_or(5000);
    let stats = analyze(log);
    let duplicates = find_duplicate_calls(log);
    let slow = find_slow_calls(log, threshold);

    FullAnalysis {
        stats,
        duplicate_calls: duplicates,
        slow_calls: slow
            .iter()
            .map(|c| SlowCallInfo {
                id: c.id.clone(),
                name: c.name.clone(),
                duration_ms: c.duration_ms,
                status: match c.status {
                    crate::parse::CallStatus::Success => "success".into(),
                    crate::parse::CallStatus::Error => "error".into(),
                    crate::parse::CallStatus::Cancelled => "cancelled".into(),
                    crate::parse::CallStatus::Pending => "pending".into(),
                },
            })
            .collect(),
        total_calls: log.total_calls,
        error_count: log.error_count,
        total_time_ms: log.total_time_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{CallStatus, ToolCall, ToolCallLog};

    fn make_log() -> ToolCallLog {
        ToolCallLog {
            trace_id: "test-1".into(),
            calls: vec![
                ToolCall {
                    id: "1".into(),
                    name: "search".into(),
                    input: serde_json::json!({"q":"hello"}),
                    output: Some(serde_json::json!({"results":[]})),
                    error: None,
                    start_time_ms: 0,
                    end_time_ms: 100,
                    duration_ms: 100,
                    status: CallStatus::Success,
                },
                ToolCall {
                    id: "2".into(),
                    name: "search".into(),
                    input: serde_json::json!({"q":"hello"}),
                    output: None,
                    error: Some("timeout".into()),
                    start_time_ms: 100,
                    end_time_ms: 200,
                    duration_ms: 100,
                    status: CallStatus::Error,
                },
                ToolCall {
                    id: "3".into(),
                    name: "read_file".into(),
                    input: serde_json::json!({"path":"/x"}),
                    output: Some(serde_json::json!({"content":"hi"})),
                    error: None,
                    start_time_ms: 200,
                    end_time_ms: 500,
                    duration_ms: 300,
                    status: CallStatus::Success,
                },
            ],
            total_time_ms: 500,
            total_calls: 3,
            error_count: 1,
        }
    }

    #[test]
    fn test_analyze() {
        let log = make_log();
        let a = analyze(&log);
        assert!((a.avg_duration_ms - 166.666).abs() < 1.0);
        assert_eq!(a.min_duration_ms, 100);
        assert_eq!(a.max_duration_ms, 300);
        assert_eq!(a.most_called_tool, "search");
        assert!((a.error_rate - 1.0 / 3.0).abs() < 0.01);
        assert!(a.total_input_bytes > 0);
        assert!(a.total_output_bytes > 0);
    }

    #[test]
    fn most_called_tool_is_empty_when_frequency_is_tied() {
        let mut log = make_log();
        log.calls.remove(1);
        log.total_calls = 2;

        let analysis = analyze(&log);

        assert_eq!(analysis.most_called_tool, "");
    }

    #[test]
    fn test_find_duplicate_calls() {
        let log = make_log();
        let dups = find_duplicate_calls(&log);
        assert!(!dups.is_empty());
        // search with same input appears twice
        assert!(dups.iter().any(|(n, _)| n.contains("search")));
    }

    #[test]
    fn duplicate_groups_for_the_same_tool_are_counted_independently() {
        let mut log = make_log();
        log.calls.extend([
            ToolCall {
                id: "4".into(),
                name: "search".into(),
                input: serde_json::json!({"q":"different"}),
                output: None,
                error: None,
                start_time_ms: 500,
                end_time_ms: 600,
                duration_ms: 100,
                status: CallStatus::Success,
            },
            ToolCall {
                id: "5".into(),
                name: "search".into(),
                input: serde_json::json!({"q":"different"}),
                output: None,
                error: None,
                start_time_ms: 600,
                end_time_ms: 700,
                duration_ms: 100,
                status: CallStatus::Success,
            },
        ]);

        let duplicates = find_duplicate_calls(&log);
        let repeated_calls: u32 = duplicates.iter().map(|(_, count)| count - 1).sum();

        assert_eq!(duplicates.len(), 2);
        assert_eq!(repeated_calls, 2);
    }

    #[test]
    fn test_find_slow_calls() {
        let log = make_log();
        let slow = find_slow_calls(&log, 200);
        assert_eq!(slow.len(), 1);
        assert_eq!(slow[0].name, "read_file");
    }

    #[test]
    fn test_empty_log_analysis() {
        let log = ToolCallLog {
            trace_id: "empty".into(),
            calls: vec![],
            total_time_ms: 0,
            total_calls: 0,
            error_count: 0,
        };
        let a = analyze(&log);
        assert_eq!(a.avg_duration_ms, 0.0);
        assert_eq!(a.most_called_tool, "");
    }

    #[test]
    fn average_duration_does_not_overflow_u64() {
        let calls = vec![
            ToolCall {
                id: "1".into(),
                name: "search".into(),
                input: serde_json::Value::Null,
                output: None,
                error: None,
                start_time_ms: 0,
                end_time_ms: u64::MAX,
                duration_ms: u64::MAX,
                status: CallStatus::Success,
            },
            ToolCall {
                id: "2".into(),
                name: "read".into(),
                input: serde_json::Value::Null,
                output: None,
                error: None,
                start_time_ms: 0,
                end_time_ms: u64::MAX,
                duration_ms: u64::MAX,
                status: CallStatus::Success,
            },
        ];
        let log = ToolCallLog {
            trace_id: "large-durations".into(),
            calls,
            total_time_ms: u64::MAX,
            total_calls: 2,
            error_count: 0,
        };

        let analysis = analyze(&log);

        assert_eq!(analysis.avg_duration_ms, u64::MAX as f64);
    }

    fn retry_call(
        id: &str,
        name: &str,
        input: serde_json::Value,
        status: CallStatus,
        start_time_ms: u64,
        end_time_ms: u64,
    ) -> ToolCall {
        ToolCall {
            id: id.into(),
            name: name.into(),
            input,
            output: None,
            error: (status == CallStatus::Error).then(|| "sensitive upstream detail".into()),
            start_time_ms,
            end_time_ms,
            duration_ms: end_time_ms - start_time_ms,
            status,
        }
    }

    #[test]
    fn detects_three_consecutive_failures_with_canonical_name_and_input() {
        let log = ToolCallLog {
            trace_id: "retry-loop".into(),
            calls: vec![
                retry_call(
                    "1",
                    " Search ",
                    serde_json::json!({"limit": 10, "query": "private"}),
                    CallStatus::Error,
                    0,
                    10,
                ),
                retry_call(
                    "2",
                    "search",
                    serde_json::json!({"query": "private", "limit": 10}),
                    CallStatus::Error,
                    10,
                    20,
                ),
                retry_call(
                    "3",
                    "SEARCH",
                    serde_json::json!({"limit": 10, "query": "private"}),
                    CallStatus::Error,
                    20,
                    30,
                ),
            ],
            total_time_ms: 30,
            total_calls: 3,
            error_count: 3,
        };

        let findings = find_retry_loop_findings(&log);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, RetryFindingKind::ConsecutiveFailures);
        assert_eq!(findings[0].tool_name, "search");
        assert_eq!(findings[0].call_count, 3);
        assert_eq!(findings[0].call_ids, ["1", "2", "3"]);
        assert!(!findings[0].message.contains("private"));
        assert!(!findings[0].message.contains("sensitive upstream detail"));
    }

    #[test]
    fn detects_overlapping_duplicate_calls() {
        let input = serde_json::json!({"path": "/private/file"});
        let log = ToolCallLog {
            trace_id: "overlap".into(),
            calls: vec![
                retry_call("1", "read_file", input.clone(), CallStatus::Success, 0, 20),
                retry_call("2", "read_file", input, CallStatus::Success, 10, 30),
            ],
            total_time_ms: 30,
            total_calls: 2,
            error_count: 0,
        };

        let findings = find_retry_loop_findings(&log);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, RetryFindingKind::OverlappingDuplicate);
        assert_eq!(findings[0].call_ids, ["1", "2"]);
        assert!(!findings[0].message.contains("/private/file"));
    }

    #[test]
    fn reports_recovery_after_repeated_failures_but_not_repeated_successes() {
        let input = serde_json::json!({"query": "same"});
        let log = ToolCallLog {
            trace_id: "recovered".into(),
            calls: vec![
                retry_call("1", "search", input.clone(), CallStatus::Error, 0, 10),
                retry_call("2", "search", input.clone(), CallStatus::Error, 10, 20),
                retry_call("3", "search", input.clone(), CallStatus::Error, 20, 30),
                retry_call("4", "search", input.clone(), CallStatus::Success, 30, 40),
                retry_call(
                    "5",
                    "other",
                    serde_json::json!({}),
                    CallStatus::Success,
                    40,
                    50,
                ),
                retry_call(
                    "6",
                    "other",
                    serde_json::json!({}),
                    CallStatus::Success,
                    50,
                    60,
                ),
                retry_call(
                    "7",
                    "other",
                    serde_json::json!({}),
                    CallStatus::Success,
                    60,
                    70,
                ),
            ],
            total_time_ms: 70,
            total_calls: 7,
            error_count: 3,
        };

        let findings = find_retry_loop_findings(&log);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, RetryFindingKind::RecoveredRetryLoop);
        assert_eq!(findings[0].call_count, 4);
        assert_eq!(findings[0].call_ids, ["1", "2", "3", "4"]);
    }
}
