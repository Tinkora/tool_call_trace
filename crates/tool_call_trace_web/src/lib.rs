use serde::Serialize;
use tool_call_trace_core::{
    CoreError, analyze_json, auto_parse_json, parse_generic_array_json, parse_openai_format_json,
    redact_log_json,
};
use wasm_bindgen::prelude::*;

/// Converts a CoreError into a JsValue error carrying a stable `code` field.
fn core_err(e: CoreError) -> JsValue {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"code".into(), &e.code().into()).ok();
    js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into()).ok();
    obj.into()
}

fn parse_and_return(result: Result<String, CoreError>) -> Result<JsValue, JsValue> {
    let json_str = result.map_err(core_err)?;
    let value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {e}")))?;
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsValue::from_str(&format!("Serialization failed: {e}")))
}

/// Parse a tool-call log in OpenAI run-steps format.
/// Returns a JS object representing the ToolCallLog.
#[wasm_bindgen]
pub fn wasm_parse_openai_format(json: &str) -> Result<JsValue, JsValue> {
    parse_and_return(parse_openai_format_json(json))
}

/// Parse a tool-call log in generic flat-array format.
/// Returns a JS object representing the ToolCallLog.
#[wasm_bindgen]
pub fn wasm_parse_generic_array(json: &str) -> Result<JsValue, JsValue> {
    parse_and_return(parse_generic_array_json(json))
}

/// Analyze a previously parsed tool-call log JSON for statistics and insights.
/// `log_json` should be the JSON string output from one of the parse functions.
/// `slow_threshold_ms` is optional; defaults to 5000ms.
#[wasm_bindgen]
pub fn wasm_analyze(log_json: &str, slow_threshold_ms: Option<u32>) -> Result<JsValue, JsValue> {
    let result = analyze_json(log_json, slow_threshold_ms.map(|v| v as u64));
    parse_and_return(result)
}

/// Redact a normalized log with a JSON `RedactionConfig` object.
#[wasm_bindgen]
pub fn wasm_redact_log(log_json: &str, config_json: &str) -> Result<JsValue, JsValue> {
    parse_and_return(redact_log_json(log_json, config_json))
}

/// Auto-detect Generic, OpenAI, OpenAI Agents, LangChain, or PydanticAI traces.
#[wasm_bindgen]
pub fn wasm_auto_parse(json: &str) -> Result<JsValue, JsValue> {
    parse_and_return(auto_parse_json(json))
}
