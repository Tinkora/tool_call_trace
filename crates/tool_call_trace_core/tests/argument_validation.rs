use serde_json::json;
use tool_call_trace_core::{
    ArgumentDiagnosticCode, parse_generic_array, parse_tool_inventory, validate_tool_arguments,
};

fn inventory() -> String {
    json!({
        "tools": [{
            "name": "search",
            "inputSchema": {
                "type": "object",
                "required": ["query"],
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1}
                },
                "additionalProperties": false
            }
        }]
    })
    .to_string()
}

#[test]
fn validates_supported_object_arguments() {
    let log = parse_generic_array(
        &json!([{
            "id": "call_1", "name": "search", "input": {"query": "rust", "limit": 2},
            "start_time_ms": 0, "end_time_ms": 1, "status": "success"
        }])
        .to_string(),
    )
    .unwrap();
    let tools = parse_tool_inventory(&inventory()).unwrap();

    assert!(validate_tool_arguments(&log, &tools).is_empty());
}

#[test]
fn reports_invalid_json_non_object_schema_mismatch_and_unknown_tool() {
    let log = parse_generic_array(&json!([
        {"id":"bad_json","name":"search","input":"{oops","start_time_ms":0,"end_time_ms":1,"status":"error"},
        {"id":"array","name":"search","input":[],"start_time_ms":1,"end_time_ms":2,"status":"error"},
        {"id":"schema","name":"search","input":{"limit":0},"start_time_ms":2,"end_time_ms":3,"status":"error"},
        {"id":"unknown","name":"missing","input":{},"start_time_ms":3,"end_time_ms":4,"status":"error"}
    ]).to_string()).unwrap();
    let tools = parse_tool_inventory(&inventory()).unwrap();

    let diagnostics = validate_tool_arguments(&log, &tools);
    let codes: Vec<_> = diagnostics.iter().map(|item| item.code).collect();
    assert!(codes.contains(&ArgumentDiagnosticCode::InvalidJson));
    assert!(codes.contains(&ArgumentDiagnosticCode::NonObject));
    assert!(codes.contains(&ArgumentDiagnosticCode::SchemaMismatch));
    assert!(codes.contains(&ArgumentDiagnosticCode::UnknownTool));
    assert!(
        diagnostics
            .iter()
            .all(|item| !item.message.contains("oops"))
    );
}

#[test]
fn correlates_and_bounds_repeated_failures() {
    let calls: Vec<_> = (0..25)
        .map(|index| {
            json!({
                "id": format!("call_{index}"), "name":"search", "input":{},
                "start_time_ms": index, "end_time_ms": index + 1, "status":"error"
            })
        })
        .collect();
    let log = parse_generic_array(&serde_json::to_string(&calls).unwrap()).unwrap();
    let tools = parse_tool_inventory(&inventory()).unwrap();

    let diagnostics = validate_tool_arguments(&log, &tools);
    let repeated = diagnostics
        .iter()
        .find(|item| item.code == ArgumentDiagnosticCode::RepeatedValidationFailure)
        .unwrap();
    assert_eq!(repeated.call_count, 25);
    assert_eq!(repeated.call_ids.len(), 20);
    assert!(repeated.call_ids_truncated);
}

#[test]
fn rejects_inventory_without_explicit_mcp_tool_shape() {
    assert!(parse_tool_inventory(r#"{"name":"search"}"#).is_err());
    assert!(parse_tool_inventory(r#"[{"name":"search","inputSchema":true}]"#).is_err());
    assert!(parse_tool_inventory(r#"[{"name":"search","inputSchema":{"oneOf":[]}}]"#).is_err());
}

#[test]
fn decodes_json_encoded_object_arguments_without_exposing_values() {
    let log = parse_generic_array(
        &json!([{
            "id": "call_1", "name": "search", "input": "{\"query\":\"secret\"}",
            "start_time_ms": 0, "end_time_ms": 1, "status": "success"
        }])
        .to_string(),
    )
    .unwrap();
    let tools = parse_tool_inventory(&inventory()).unwrap();

    assert!(validate_tool_arguments(&log, &tools).is_empty());
}

#[test]
fn rejects_properties_when_additional_properties_is_false_without_a_map() {
    let log = parse_generic_array(
        &json!([{
            "id": "call_1", "name": "empty", "input": {"unexpected": true},
            "start_time_ms": 0, "end_time_ms": 1, "status": "success"
        }])
        .to_string(),
    )
    .unwrap();
    let tools = parse_tool_inventory(
        r#"[{"name":"empty","inputSchema":{"type":"object","additionalProperties":false}}]"#,
    )
    .unwrap();

    let diagnostics = validate_tool_arguments(&log, &tools);
    assert_eq!(diagnostics[0].code, ArgumentDiagnosticCode::SchemaMismatch);
}
