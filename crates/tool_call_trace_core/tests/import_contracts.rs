use tool_call_trace_core::{
    CallStatus, parse_agent_trace, parse_langchain_format, parse_openai_agents_format,
    parse_pydantic_ai_logfire_format,
};

const OPENAI_AGENTS: &str = include_str!("fixtures/openai_agents_function_spans.json");
const LANGCHAIN: &str = include_str!("fixtures/langchain_tool_run.json");
const PYDANTIC_AI: &str = include_str!("fixtures/pydantic_ai_logfire_spans.json");

#[test]
fn openai_agents_fixture_parses_a_function_span() {
    let log = parse_openai_agents_format(OPENAI_AGENTS).unwrap();

    assert_eq!(log.trace_id, "trace_openai_public_fixture");
    assert_eq!(log.total_time_ms, 125);
    assert_eq!(log.calls.len(), 1);
    assert_eq!(log.calls[0].id, "span_openai_search_01");
    assert_eq!(log.calls[0].name, "search_docs");
    assert_eq!(log.calls[0].input["query"], "Rust WASM");
    assert_eq!(log.calls[0].output.as_ref().unwrap()["matches"], 2);
    assert_eq!(log.calls[0].status, CallStatus::Success);
}

#[test]
fn langchain_fixture_preserves_structured_tool_input() {
    let log = parse_langchain_format(LANGCHAIN).unwrap();

    assert_eq!(log.trace_id, "1f6a5b0d-ecfe-4f37-9f1d-a1b6c6c98901");
    assert_eq!(log.total_time_ms, 90);
    assert_eq!(log.calls[0].name, "shell");
    assert_eq!(log.calls[0].input["command"], "printf hello");
    assert_eq!(log.calls[0].output.as_ref().unwrap()["output"], "hello");
}

#[test]
fn pydantic_ai_fixture_parses_logfire_tool_attributes() {
    let log = parse_pydantic_ai_logfire_format(PYDANTIC_AI).unwrap();

    assert_eq!(log.trace_id, "1");
    assert_eq!(log.total_time_ms, 40);
    assert_eq!(log.calls[0].id, "pyd_ai_call_01");
    assert_eq!(log.calls[0].name, "add_numbers");
    assert_eq!(log.calls[0].input["x"], 42);
    assert_eq!(log.calls[0].output.as_ref().unwrap(), 84);
}

#[test]
fn openai_agents_and_langchain_unfinished_calls_remain_pending() {
    let openai = r#"{
      "data": [{
        "object": "trace.span",
        "id": "span_pending",
        "trace_id": "trace_pending",
        "started_at": "2026-08-11T09:00:00Z",
        "ended_at": null,
        "span_data": {"type":"function","name":"wait","input":"{}","output":null},
        "error": null
      }]
    }"#;
    let langchain = r#"{
      "id":"run_pending",
      "name":"wait",
      "start_time":"2026-08-11T09:00:00Z",
      "end_time":null,
      "inputs":{},
      "outputs":null,
      "error":null,
      "run_type":"tool",
      "trace_id":"trace_pending",
      "child_runs":[]
    }"#;

    for log in [
        parse_openai_agents_format(openai).unwrap(),
        parse_langchain_format(langchain).unwrap(),
    ] {
        assert_eq!(log.calls[0].status, CallStatus::Pending);
        assert_eq!(log.calls[0].duration_ms, 0);
    }
}

#[test]
fn pydantic_ai_legacy_attributes_and_exception_events_map_to_error() {
    let input = r#"[{
      "name":"running tool",
      "context":{"trace_id":1,"span_id":9,"is_remote":false},
      "parent":null,
      "start_time":4000000000,
      "end_time":4050000000,
      "attributes":{
        "gen_ai.operation.name":"execute_tool",
        "gen_ai.tool.name":"legacy_tool",
        "gen_ai.tool.call.id":"pyd_ai_legacy",
        "tool_arguments":{"x":1},
        "tool_response":{"message":"failed"},
        "logfire.level_num":17
      },
      "events":[{"name":"exception","timestamp":4040000000,"attributes":{"exception.message":"failed"}}]
    }]"#;

    let log = parse_pydantic_ai_logfire_format(input).unwrap();
    assert_eq!(log.calls[0].input["x"], 1);
    assert_eq!(log.calls[0].output.as_ref().unwrap()["message"], "failed");
    assert_eq!(log.calls[0].status, CallStatus::Error);
    assert_eq!(log.calls[0].error.as_deref(), Some("failed"));
}

#[test]
fn auto_detection_recognizes_each_pinned_sdk_contract() {
    for (input, expected_name) in [
        (OPENAI_AGENTS, "search_docs"),
        (LANGCHAIN, "shell"),
        (PYDANTIC_AI, "add_numbers"),
    ] {
        let log = parse_agent_trace(input).unwrap();
        assert_eq!(log.calls[0].name, expected_name);
    }
}
