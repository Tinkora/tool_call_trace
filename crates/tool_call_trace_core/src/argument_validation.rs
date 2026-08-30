use crate::error::CoreError;
use crate::parse::{ToolCallLog, validate_input_size};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

const MAX_CALL_DIAGNOSTICS: usize = 2_000;
const MAX_REPEATED_DIAGNOSTICS: usize = 100;
const MAX_DIAGNOSTIC_CALL_IDS: usize = 20;
const REPEATED_FAILURE_THRESHOLD: u32 = 3;
const MAX_SCHEMA_DEPTH: usize = 32;

/// An MCP tool name and its declared input schema.
#[derive(Clone, Debug)]
pub struct ToolSchema {
    name: String,
    input_schema: Value,
}

/// Stable argument-validation diagnostic codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ArgumentDiagnosticCode {
    #[serde(rename = "ARG001_INVALID_JSON")]
    InvalidJson,
    #[serde(rename = "ARG002_NON_OBJECT")]
    NonObject,
    #[serde(rename = "ARG003_SCHEMA_MISMATCH")]
    SchemaMismatch,
    #[serde(rename = "ARG004_UNKNOWN_TOOL")]
    UnknownTool,
    #[serde(rename = "ARG005_REPEATED_VALIDATION_FAILURE")]
    RepeatedValidationFailure,
}

impl ArgumentDiagnosticCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidJson => "ARG001_INVALID_JSON",
            Self::NonObject => "ARG002_NON_OBJECT",
            Self::SchemaMismatch => "ARG003_SCHEMA_MISMATCH",
            Self::UnknownTool => "ARG004_UNKNOWN_TOOL",
            Self::RepeatedValidationFailure => "ARG005_REPEATED_VALIDATION_FAILURE",
        }
    }
}

/// A bounded diagnostic that never includes argument values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgumentDiagnostic {
    pub code: ArgumentDiagnosticCode,
    pub tool_name: String,
    pub message: String,
    pub call_count: u32,
    pub call_ids: Vec<String>,
    pub call_ids_truncated: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum InventoryEnvelope {
    Result { tools: Vec<RawTool> },
    Array(Vec<RawTool>),
}

#[derive(Deserialize)]
struct RawTool {
    name: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

/// Parse either an MCP `tools/list` result object or its `tools` array.
pub fn parse_tool_inventory(json: &str) -> Result<Vec<ToolSchema>, CoreError> {
    validate_input_size(json)?;
    let envelope: InventoryEnvelope = serde_json::from_str(json)
        .map_err(|error| CoreError::InvalidFormat(format!("invalid tool inventory: {error}")))?;
    let tools = match envelope {
        InventoryEnvelope::Result { tools } | InventoryEnvelope::Array(tools) => tools,
    };
    let mut names = HashSet::new();
    let mut parsed = Vec::with_capacity(tools.len());
    for tool in tools {
        let name = tool.name.trim();
        if name.is_empty() {
            return Err(CoreError::InvalidFormat(
                "tool inventory contains an empty name".into(),
            ));
        }
        if !tool.input_schema.is_object() {
            return Err(CoreError::InvalidFormat(format!(
                "inputSchema must be an object for {}",
                safe_label(name)
            )));
        }
        validate_supported_schema(&tool.input_schema, 0).map_err(|message| {
            CoreError::InvalidFormat(format!(
                "unsupported inputSchema for {}: {message}",
                safe_label(name)
            ))
        })?;
        if !names.insert(name.to_string()) {
            return Err(CoreError::InvalidFormat(format!(
                "duplicate tool name in inventory: {}",
                safe_label(name)
            )));
        }
        parsed.push(ToolSchema {
            name: name.to_string(),
            input_schema: tool.input_schema,
        });
    }
    Ok(parsed)
}

fn validate_supported_schema(schema: &Value, depth: usize) -> Result<(), String> {
    if depth > MAX_SCHEMA_DEPTH {
        return Err("schema nesting exceeds the supported depth".into());
    }
    let object = schema
        .as_object()
        .ok_or_else(|| "nested schemas must be objects".to_string())?;
    const SUPPORTED: &[&str] = &[
        "type",
        "required",
        "properties",
        "additionalProperties",
        "items",
        "enum",
        "minimum",
        "maximum",
        "$schema",
        "title",
        "description",
        "default",
        "examples",
        "deprecated",
        "readOnly",
        "writeOnly",
    ];
    if let Some(keyword) = object.keys().find(|key| !SUPPORTED.contains(&key.as_str())) {
        return Err(format!("keyword {} is not supported", safe_label(keyword)));
    }
    if let Some(kind) = object.get("type") {
        let kind = kind
            .as_str()
            .ok_or_else(|| "type must be a single string".to_string())?;
        if ![
            "object", "array", "string", "number", "integer", "boolean", "null",
        ]
        .contains(&kind)
        {
            return Err(format!("type {} is not supported", safe_label(kind)));
        }
    }
    if let Some(required) = object.get("required") {
        let required = required
            .as_array()
            .ok_or_else(|| "required must be an array".to_string())?;
        if required.iter().any(|name| !name.is_string()) {
            return Err("required entries must be strings".into());
        }
    }
    if let Some(properties) = object.get("properties") {
        let properties = properties
            .as_object()
            .ok_or_else(|| "properties must be an object".to_string())?;
        for child in properties.values() {
            validate_supported_schema(child, depth + 1)?;
        }
    }
    if let Some(additional) = object.get("additionalProperties")
        && !additional.is_boolean()
    {
        return Err("additionalProperties must be a boolean".into());
    }
    if let Some(items) = object.get("items") {
        validate_supported_schema(items, depth + 1)?;
    }
    if let Some(values) = object.get("enum")
        && !values.is_array()
    {
        return Err("enum must be an array".into());
    }
    for keyword in ["minimum", "maximum"] {
        if let Some(value) = object.get(keyword)
            && !value.is_number()
        {
            return Err(format!("{keyword} must be a number"));
        }
    }
    Ok(())
}

/// Validate observed arguments against the deliberately small documented schema subset.
pub fn validate_tool_arguments(
    log: &ToolCallLog,
    inventory: &[ToolSchema],
) -> Vec<ArgumentDiagnostic> {
    let tools: HashMap<&str, &Value> = inventory
        .iter()
        .map(|tool| (tool.name.as_str(), &tool.input_schema))
        .collect();
    let mut diagnostics = Vec::new();
    let mut groups: HashMap<(String, ArgumentDiagnosticCode, String), (u32, Vec<String>)> =
        HashMap::new();

    for call in &log.calls {
        let failure = match tools.get(call.name.as_str()) {
            None => Some((
                ArgumentDiagnosticCode::UnknownTool,
                "tool name is not present in the supplied inventory".to_string(),
            )),
            Some(schema) => match decode_object(&call.input) {
                Err(code) => Some((
                    code,
                    match code {
                        ArgumentDiagnosticCode::InvalidJson => {
                            "argument string is not valid JSON".to_string()
                        }
                        _ => "decoded arguments must be a JSON object".to_string(),
                    },
                )),
                Ok(arguments) => validate_schema(&arguments, schema, 0)
                    .map(|message| (ArgumentDiagnosticCode::SchemaMismatch, message)),
            },
        };

        if let Some((code, message)) = failure {
            let key = (call.name.clone(), code, message.clone());
            let group = groups.entry(key).or_insert_with(|| (0, Vec::new()));
            group.0 = group.0.saturating_add(1);
            if group.1.len() < MAX_DIAGNOSTIC_CALL_IDS {
                group.1.push(call.id.clone());
            }
            if diagnostics.len() < MAX_CALL_DIAGNOSTICS {
                diagnostics.push(ArgumentDiagnostic {
                    code,
                    tool_name: call.name.clone(),
                    message,
                    call_count: 1,
                    call_ids: vec![call.id.clone()],
                    call_ids_truncated: false,
                });
            }
        }
    }

    let mut repeated: Vec<_> = groups
        .into_iter()
        .filter(|(_, (count, _))| *count >= REPEATED_FAILURE_THRESHOLD)
        .collect();
    repeated.sort_by(|a, b| a.0.cmp(&b.0));
    for ((tool_name, code, _), (call_count, call_ids)) in
        repeated.into_iter().take(MAX_REPEATED_DIAGNOSTICS)
    {
        diagnostics.push(ArgumentDiagnostic {
            code: ArgumentDiagnosticCode::RepeatedValidationFailure,
            tool_name,
            message: format!(
                "the same {} validation failure occurred {call_count} times",
                code.as_str()
            ),
            call_count,
            call_ids_truncated: call_count as usize > call_ids.len(),
            call_ids,
        });
    }
    diagnostics
}

fn decode_object(input: &Value) -> Result<Value, ArgumentDiagnosticCode> {
    match input {
        Value::Object(_) => Ok(input.clone()),
        Value::String(encoded) => {
            let decoded: Value =
                serde_json::from_str(encoded).map_err(|_| ArgumentDiagnosticCode::InvalidJson)?;
            if decoded.is_object() {
                Ok(decoded)
            } else {
                Err(ArgumentDiagnosticCode::NonObject)
            }
        }
        _ => Err(ArgumentDiagnosticCode::NonObject),
    }
}

fn validate_schema(value: &Value, schema: &Value, depth: usize) -> Option<String> {
    if depth > MAX_SCHEMA_DEPTH {
        return Some("schema nesting exceeds the supported depth".into());
    }
    let object = schema.as_object()?;
    if let Some(expected) = object.get("type").and_then(Value::as_str) {
        let matches = match expected {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            _ => true,
        };
        if !matches {
            return Some(format!("value does not match declared type {expected}"));
        }
    }
    if let Some(allowed) = object.get("enum").and_then(Value::as_array)
        && !allowed.contains(value)
    {
        return Some("value is not in the declared enum".into());
    }
    if let Some(instance) = value.as_object() {
        if let Some(required) = object.get("required").and_then(Value::as_array) {
            for name in required.iter().filter_map(Value::as_str) {
                if !instance.contains_key(name) {
                    return Some(format!("required property {} is missing", safe_label(name)));
                }
            }
        }
        let properties = object.get("properties").and_then(Value::as_object);
        for (name, child) in instance {
            if let Some(child_schema) = properties.and_then(|schemas| schemas.get(name)) {
                if let Some(reason) = validate_schema(child, child_schema, depth + 1) {
                    return Some(format!("property {}: {reason}", safe_label(name)));
                }
            } else if object.get("additionalProperties") == Some(&Value::Bool(false)) {
                return Some(format!("property {} is not allowed", safe_label(name)));
            }
        }
    }
    if let (Some(items), Some(values)) = (object.get("items"), value.as_array()) {
        for item in values {
            if let Some(reason) = validate_schema(item, items, depth + 1) {
                return Some(format!("array item: {reason}"));
            }
        }
    }
    if let Some(number) = value.as_f64() {
        if let Some(minimum) = object.get("minimum").and_then(Value::as_f64)
            && number < minimum
        {
            return Some("number is below the declared minimum".into());
        }
        if let Some(maximum) = object.get("maximum").and_then(Value::as_f64)
            && number > maximum
        {
            return Some("number exceeds the declared maximum".into());
        }
    }
    None
}

fn safe_label(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(120)
        .collect()
}
