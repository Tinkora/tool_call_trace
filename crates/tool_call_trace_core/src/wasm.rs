use crate::analyze::full_analyze;
use crate::error::CoreError;
use crate::import::parse_agent_trace;
use crate::parse::{self, ToolCallLog};

/// Parse a tool-call log in OpenAI run-steps format.
/// Returns the ToolCallLog serialized as a JSON string.
pub fn parse_openai_format_json(json: &str) -> Result<String, CoreError> {
    let log = parse::parse_openai_format(json)?;
    serde_json::to_string(&log).map_err(|e| CoreError::ParseError(e.to_string()))
}

/// Parse a tool-call log in generic flat-array format.
/// Returns the ToolCallLog serialized as a JSON string.
pub fn parse_generic_array_json(json: &str) -> Result<String, CoreError> {
    let log = parse::parse_generic_array(json)?;
    serde_json::to_string(&log).map_err(|e| CoreError::ParseError(e.to_string()))
}

/// Analyze a previously parsed tool-call log JSON for statistics and insights.
/// `log_json` should be the JSON output from one of the parse functions.
/// `slow_threshold_ms` is optional; defaults to 5000ms.
pub fn analyze_json(log_json: &str, slow_threshold_ms: Option<u64>) -> Result<String, CoreError> {
    let log: ToolCallLog =
        serde_json::from_str(log_json).map_err(|e| CoreError::ParseError(e.to_string()))?;
    let analysis = full_analyze(&log, slow_threshold_ms);
    serde_json::to_string(&analysis).map_err(|e| CoreError::ParseError(e.to_string()))
}

/// Auto-detect the format of a tool-call log and parse it.
/// Tries OpenAI, then Generic Array format.
pub fn auto_parse_json(json: &str) -> Result<String, CoreError> {
    let log = parse_agent_trace(json)?;
    serde_json::to_string(&log).map_err(|e| CoreError::ParseError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_roundtrip() {
        let input = r#"[
            {"id":"1","name":"search","input":{"q":"x"},"start_time_ms":0,"end_time_ms":100,"status":"success"}
        ]"#;
        let json = parse_generic_array_json(input).unwrap();
        let log: ToolCallLog = serde_json::from_str(&json).unwrap();
        assert_eq!(log.total_calls, 1);
    }

    #[test]
    fn test_analyze_json() {
        let input = r#"[
            {"id":"1","name":"search","input":{"q":"x"},"start_time_ms":0,"end_time_ms":100,"status":"success"},
            {"id":"2","name":"read","input":{"p":"y"},"start_time_ms":100,"end_time_ms":300,"status":"success"}
        ]"#;
        let log_json = parse_generic_array_json(input).unwrap();
        let analysis_json = analyze_json(&log_json, None).unwrap();
        let analysis: serde_json::Value = serde_json::from_str(&analysis_json).unwrap();
        assert_eq!(analysis["total_calls"].as_u64().unwrap(), 2);
        assert_eq!(analysis["error_count"].as_u64().unwrap(), 0);
        assert_eq!(analysis["total_time_ms"].as_u64().unwrap(), 300);
    }

    #[test]
    fn test_auto_parse_openai() {
        let input = r#"{
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
                            "arguments": "{\"query\":\"hello\"}"
                        }
                    }]
                },
                "created_at": 1715000000,
                "completed_at": 1715000005,
                "status": "completed"
            }]
        }"#;
        let json = auto_parse_json(input).unwrap();
        let log: ToolCallLog = serde_json::from_str(&json).unwrap();
        assert_eq!(log.calls[0].name, "search");
    }

    #[test]
    fn auto_parse_rejects_anthropic_blocks_without_timing_data() {
        let input = r#"{
            "model": "claude-sonnet-4-20250514",
            "role": "assistant",
            "content": [{
                "type": "tool_use",
                "id": "toolu_001",
                "name": "search",
                "input": {"query": "Rust"}
            }]
        }"#;

        assert!(auto_parse_json(input).is_err());
    }
}
