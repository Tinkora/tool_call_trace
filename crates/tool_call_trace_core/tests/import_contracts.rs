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

    assert_eq!(log.trace_id, "8f2efebf74bd4eaf8c7fc21969941901");
    assert_eq!(log.total_time_ms, 40);
    assert_eq!(log.calls[0].id, "pyd_ai_call_01");
    assert_eq!(log.calls[0].name, "add_numbers");
    assert_eq!(log.calls[0].input["x"], 42);
    assert_eq!(log.calls[0].output.as_ref().unwrap(), 84);
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
