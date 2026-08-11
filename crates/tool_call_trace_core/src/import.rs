use crate::error::CoreError;
use crate::parse::{
    CallStatus, ToolCall, ToolCallLog, normalize_timestamps, parse_generic_array,
    parse_openai_format, validate_call_count, validate_input_size,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Debug, Deserialize)]
struct OpenAIAgentsSpan {
    id: String,
    trace_id: String,
    started_at: String,
    ended_at: Option<String>,
    span_data: OpenAIAgentsSpanData,
    error: Option<OpenAIAgentsSpanError>,
}

#[derive(Debug, Deserialize)]
struct OpenAIAgentsSpanData {
    #[serde(rename = "type")]
    span_type: String,
    name: Option<String>,
    input: Option<String>,
    output: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIAgentsSpanError {
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LangChainRun {
    id: String,
    name: String,
    start_time: String,
    end_time: Option<String>,
    inputs: Value,
    outputs: Option<Value>,
    error: Option<String>,
    run_type: String,
    trace_id: String,
    #[serde(default)]
    child_runs: Vec<LangChainRun>,
}

#[derive(Debug, Deserialize)]
struct PydanticSpan {
    context: PydanticSpanContext,
    start_time: PydanticTimestamp,
    end_time: Option<PydanticTimestamp>,
    attributes: serde_json::Map<String, Value>,
    status: Option<PydanticSpanStatus>,
    #[serde(default)]
    events: Vec<PydanticSpanEvent>,
}

#[derive(Debug, Deserialize)]
struct PydanticSpanContext {
    trace_id: PydanticId,
    span_id: PydanticId,
}

#[derive(Debug, Deserialize)]
struct PydanticSpanStatus {
    status_code: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PydanticSpanEvent {
    name: String,
    #[serde(default)]
    attributes: serde_json::Map<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PydanticId {
    String(String),
    Unsigned(u64),
    Signed(i64),
}

impl PydanticId {
    fn as_string(&self) -> String {
        match self {
            Self::String(value) => value.clone(),
            Self::Unsigned(value) => value.to_string(),
            Self::Signed(value) => value.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PydanticTimestamp {
    Nanoseconds(u64),
    Rfc3339(String),
}

fn parse_rfc3339_ms(value: &str, field: &str, id: &str) -> Result<u64, CoreError> {
    let timestamp = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| CoreError::InvalidFormat(format!("{field} must be RFC 3339 for {id}")))?;
    let milliseconds = timestamp.unix_timestamp_nanos() / 1_000_000;
    u64::try_from(milliseconds)
        .map_err(|_| CoreError::InvalidFormat(format!("{field} precedes Unix epoch for {id}")))
}

fn parse_pydantic_timestamp_ms(
    value: &PydanticTimestamp,
    field: &str,
    id: &str,
) -> Result<u64, CoreError> {
    match value {
        PydanticTimestamp::Nanoseconds(value) => Ok(value / 1_000_000),
        PydanticTimestamp::Rfc3339(value) => parse_rfc3339_ms(value, field, id),
    }
}

fn parse_json_string(value: Option<String>) -> Value {
    match value {
        Some(value) => serde_json::from_str(&value).unwrap_or(Value::String(value)),
        None => Value::Null,
    }
}

fn parse_optional_json_string(value: Option<String>) -> Option<Value> {
    value.map(|value| serde_json::from_str(&value).unwrap_or(Value::String(value)))
}

fn finish_log(trace_id: String, mut calls: Vec<ToolCall>) -> Result<ToolCallLog, CoreError> {
    if calls.is_empty() {
        return Err(CoreError::EmptyLog);
    }
    validate_call_count(calls.len())?;

    let mut seen_ids = HashSet::with_capacity(calls.len());
    let mut min_time = u64::MAX;
    let mut max_time = 0;
    let mut error_count = 0;
    for call in &calls {
        if call.id.trim().is_empty() {
            return Err(CoreError::InvalidFormat(
                "tool-call id must not be empty".into(),
            ));
        }
        if call.name.trim().is_empty() {
            return Err(CoreError::InvalidFormat(format!(
                "tool name must not be empty for {}",
                call.id
            )));
        }
        if !seen_ids.insert(call.id.clone()) {
            return Err(CoreError::InvalidFormat(format!(
                "duplicate tool-call id: {}",
                call.id
            )));
        }
        if call.end_time_ms < call.start_time_ms {
            return Err(CoreError::InvalidFormat(format!(
                "end time precedes start time for {}",
                call.id
            )));
        }
        min_time = min_time.min(call.start_time_ms);
        max_time = max_time.max(call.end_time_ms);
        if call.status == CallStatus::Error {
            error_count += 1;
        }
    }

    calls.sort_by_key(|call| call.start_time_ms);
    let total_time_ms = max_time.saturating_sub(min_time);
    normalize_timestamps(&mut calls, min_time);

    Ok(ToolCallLog {
        trace_id,
        total_calls: calls.len() as u32,
        total_time_ms,
        error_count,
        calls,
    })
}

/// Parses exported `function` spans from the OpenAI Agents SDK.
pub fn parse_openai_agents_format(json: &str) -> Result<ToolCallLog, CoreError> {
    validate_input_size(json)?;
    let value: Value =
        serde_json::from_str(json).map_err(|error| CoreError::InvalidFormat(error.to_string()))?;
    let spans: Vec<OpenAIAgentsSpan> = if let Some(data) = value.get("data") {
        serde_json::from_value(data.clone())
            .map_err(|error| CoreError::InvalidFormat(error.to_string()))?
    } else {
        serde_json::from_value(value)
            .map_err(|error| CoreError::InvalidFormat(error.to_string()))?
    };
    let mut calls = Vec::new();
    let mut trace_id = None;

    for span in spans
        .into_iter()
        .filter(|span| span.span_data.span_type == "function")
    {
        let name = span.span_data.name.ok_or_else(|| {
            CoreError::InvalidFormat(format!("function name is required for {}", span.id))
        })?;
        match &trace_id {
            Some(expected) if expected != &span.trace_id => {
                return Err(CoreError::InvalidFormat(
                    "OpenAI Agents input contains multiple trace IDs".into(),
                ));
            }
            None => trace_id = Some(span.trace_id.clone()),
            _ => {}
        }
        let start_time_ms = parse_rfc3339_ms(&span.started_at, "started_at", &span.id)?;
        let end_time_ms = span
            .ended_at
            .as_deref()
            .map(|value| parse_rfc3339_ms(value, "ended_at", &span.id))
            .transpose()?
            .unwrap_or(start_time_ms);
        let error = span.error.and_then(|error| error.message);
        let status = if error.is_some() {
            CallStatus::Error
        } else if span.ended_at.is_none() {
            CallStatus::Pending
        } else {
            CallStatus::Success
        };
        calls.push(ToolCall {
            id: span.id,
            name,
            input: parse_json_string(span.span_data.input),
            output: parse_optional_json_string(span.span_data.output),
            error,
            start_time_ms,
            end_time_ms,
            duration_ms: end_time_ms.saturating_sub(start_time_ms),
            status,
        });
    }

    finish_log(trace_id.unwrap_or_default(), calls)
}

fn collect_langchain_calls(
    run: LangChainRun,
    trace_id: &mut Option<String>,
    calls: &mut Vec<ToolCall>,
) -> Result<(), CoreError> {
    if run.run_type == "tool" {
        match trace_id.as_ref() {
            Some(expected) if expected != &run.trace_id => {
                return Err(CoreError::InvalidFormat(
                    "LangChain input contains multiple trace IDs".into(),
                ));
            }
            None => *trace_id = Some(run.trace_id.clone()),
            _ => {}
        }
        let start_time_ms = parse_rfc3339_ms(&run.start_time, "start_time", &run.id)?;
        let end_time_ms = run
            .end_time
            .as_deref()
            .map(|value| parse_rfc3339_ms(value, "end_time", &run.id))
            .transpose()?
            .unwrap_or(start_time_ms);
        let status = if run.error.is_some() {
            CallStatus::Error
        } else if run.end_time.is_none() {
            CallStatus::Pending
        } else {
            CallStatus::Success
        };
        calls.push(ToolCall {
            id: run.id,
            name: run.name,
            input: run.inputs,
            output: run.outputs,
            error: run.error,
            start_time_ms,
            end_time_ms,
            duration_ms: end_time_ms.saturating_sub(start_time_ms),
            status,
        });
    }

    for child in run.child_runs {
        collect_langchain_calls(child, trace_id, calls)?;
    }
    Ok(())
}

fn deserialize_langchain_runs(json: &str) -> Result<Vec<LangChainRun>, CoreError> {
    let value: Value =
        serde_json::from_str(json).map_err(|error| CoreError::InvalidFormat(error.to_string()))?;
    if let Some(runs) = value.get("runs") {
        serde_json::from_value(runs.clone())
            .map_err(|error| CoreError::InvalidFormat(error.to_string()))
    } else if value.is_array() {
        serde_json::from_value(value).map_err(|error| CoreError::InvalidFormat(error.to_string()))
    } else {
        serde_json::from_value(value)
            .map(|run| vec![run])
            .map_err(|error| CoreError::InvalidFormat(error.to_string()))
    }
}

/// Parses serialized LangChain `Run` records whose type is `tool`.
pub fn parse_langchain_format(json: &str) -> Result<ToolCallLog, CoreError> {
    validate_input_size(json)?;
    let runs = deserialize_langchain_runs(json)?;
    let mut calls = Vec::new();
    let mut trace_id = None;
    for run in runs {
        collect_langchain_calls(run, &mut trace_id, &mut calls)?;
    }
    finish_log(trace_id.unwrap_or_default(), calls)
}

fn required_attribute_string(
    attributes: &serde_json::Map<String, Value>,
    key: &str,
    span_id: &str,
) -> Result<String, CoreError> {
    attributes
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| CoreError::InvalidFormat(format!("{key} is required for span {span_id}")))
}

fn pydantic_attribute_value(
    attributes: &serde_json::Map<String, Value>,
    primary: &str,
    legacy: &str,
) -> Option<Value> {
    attributes
        .get(primary)
        .or_else(|| attributes.get(legacy))
        .cloned()
        .map(|value| match value {
            Value::String(text) => serde_json::from_str(&text).unwrap_or(Value::String(text)),
            value => value,
        })
}

/// Parses PydanticAI tool spans exported by Logfire's in-memory exporter.
pub fn parse_pydantic_ai_logfire_format(json: &str) -> Result<ToolCallLog, CoreError> {
    validate_input_size(json)?;
    let spans: Vec<PydanticSpan> =
        serde_json::from_str(json).map_err(|error| CoreError::InvalidFormat(error.to_string()))?;
    let mut calls = Vec::new();
    let mut trace_id = None;

    for span in spans.into_iter().filter(|span| {
        span.attributes
            .get("gen_ai.operation.name")
            .and_then(Value::as_str)
            == Some("execute_tool")
    }) {
        let span_id = span.context.span_id.as_string();
        let current_trace_id = span.context.trace_id.as_string();
        match trace_id.as_ref() {
            Some(expected) if expected != &current_trace_id => {
                return Err(CoreError::InvalidFormat(
                    "PydanticAI input contains multiple trace IDs".into(),
                ));
            }
            None => trace_id = Some(current_trace_id),
            _ => {}
        }
        let id = required_attribute_string(&span.attributes, "gen_ai.tool.call.id", &span_id)?;
        let name = required_attribute_string(&span.attributes, "gen_ai.tool.name", &span_id)?;
        let start_time_ms = parse_pydantic_timestamp_ms(&span.start_time, "start_time", &id)?;
        let end_time_ms = span
            .end_time
            .as_ref()
            .map(|value| parse_pydantic_timestamp_ms(value, "end_time", &id))
            .transpose()?
            .unwrap_or(start_time_ms);
        let input = pydantic_attribute_value(
            &span.attributes,
            "gen_ai.tool.call.arguments",
            "tool_arguments",
        );
        let output =
            pydantic_attribute_value(&span.attributes, "gen_ai.tool.call.result", "tool_response");
        let status_error = span
            .status
            .as_ref()
            .and_then(|status| status.status_code.as_deref())
            == Some("ERROR");
        let level_error = span
            .attributes
            .get("logfire.level_num")
            .and_then(Value::as_i64)
            .is_some_and(|level| level >= 17);
        let exception_error = span.events.iter().any(|event| event.name == "exception");
        let is_error = status_error || level_error || exception_error;
        let exception_message = span
            .events
            .iter()
            .find(|event| event.name == "exception")
            .and_then(|event| event.attributes.get("exception.message"))
            .and_then(Value::as_str)
            .filter(|message| !message.is_empty())
            .map(str::to_owned);
        let output_error = output
            .as_ref()
            .and_then(Value::as_str)
            .filter(|message| !message.is_empty())
            .map(str::to_owned);
        let error = is_error
            .then(|| span.status.and_then(|status| status.description))
            .flatten()
            .or(exception_message)
            .or(output_error)
            .or_else(|| is_error.then(|| "PydanticAI tool span failed".to_owned()));
        let is_deferred = span
            .attributes
            .get("pydantic_ai.tool.deferral.name")
            .is_some();
        calls.push(ToolCall {
            id,
            name,
            input: input.unwrap_or(Value::Null),
            output,
            error,
            start_time_ms,
            end_time_ms,
            duration_ms: end_time_ms.saturating_sub(start_time_ms),
            status: if is_error {
                CallStatus::Error
            } else if is_deferred || span.end_time.is_none() {
                CallStatus::Pending
            } else {
                CallStatus::Success
            },
        });
    }

    finish_log(trace_id.unwrap_or_default(), calls)
}

fn first_array_object(value: &Value) -> Option<&serde_json::Map<String, Value>> {
    value.as_array()?.first()?.as_object()
}

/// Detects a supported trace contract and parses it without ambiguous fallback.
pub fn parse_agent_trace(json: &str) -> Result<ToolCallLog, CoreError> {
    validate_input_size(json)?;
    let value: Value =
        serde_json::from_str(json).map_err(|error| CoreError::ParseError(error.to_string()))?;

    if value
        .get("data")
        .and_then(Value::as_array)
        .and_then(|data| data.first())
        .and_then(Value::as_object)
        .is_some_and(|first| {
            first.get("span_data").is_some()
                || first.get("object") == Some(&Value::String("trace.span".into()))
        })
    {
        return parse_openai_agents_format(json);
    }
    if value.get("data").is_some() {
        return parse_openai_format(json);
    }
    if value.get("run_type").is_some() || value.get("runs").is_some() {
        return parse_langchain_format(json);
    }
    if let Some(first) = first_array_object(&value) {
        if first.get("span_data").is_some()
            || first.get("object") == Some(&Value::String("trace.span".into()))
        {
            return parse_openai_agents_format(json);
        }
        if first.get("run_type").is_some() {
            return parse_langchain_format(json);
        }
        if first.get("attributes").is_some() && first.get("context").is_some() {
            return parse_pydantic_ai_logfire_format(json);
        }
    }
    parse_generic_array(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognized_openai_agents_shape_does_not_fall_back() {
        let input = r#"[{"object":"trace.span","span_data":{"type":"function"}}]"#;
        let error = parse_agent_trace(input).unwrap_err();
        assert!(matches!(error, CoreError::InvalidFormat(_)));
    }

    #[test]
    fn imported_formats_share_the_line_limit() {
        let input = format!("[{}]", "\n".repeat(crate::parse::MAX_INPUT_LINES));
        let error = parse_openai_agents_format(&input).unwrap_err();
        assert!(error.to_string().contains("line limit"));
    }
}
