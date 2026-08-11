# Agent SDK Fixture Sources

These minimized fixtures preserve the public field structure and semantics of
the upstream SDK contracts while replacing generated identifiers and ordinary
example values with deterministic test data. They contain no credentials or
personal data.

## OpenAI Agents

- Repository: <https://github.com/openai/openai-agents-python>
- Commit: `8ecdac5947b0ed9f7c08e2b4d67a038840f5d5e8`
- Sources:
  [`src/agents/tracing/spans.py`](https://github.com/openai/openai-agents-python/blob/8ecdac5947b0ed9f7c08e2b4d67a038840f5d5e8/src/agents/tracing/spans.py#L396-L423)
  and
  [`src/agents/tracing/span_data.py`](https://github.com/openai/openai-agents-python/blob/8ecdac5947b0ed9f7c08e2b4d67a038840f5d5e8/src/agents/tracing/span_data.py#L135-L166)
- Contract: exported `trace.span` records with `function` span data.
- Source kind: adapted exporter payload. The official processor wraps exported
  spans in `{"data": [...]}`; generated IDs, timestamps, and ordinary values
  are deterministic replacements.
- License: MIT.

## LangChain

- Repository: <https://github.com/langchain-ai/langchain>
- Commit: `f78df6d9772305e29ac07ae5508b468f56a4bcd3`
- Source:
  [`libs/core/tests/unit_tests/tracers/test_base_tracer.py`](https://github.com/langchain-ai/langchain/blob/f78df6d9772305e29ac07ae5508b468f56a4bcd3/libs/core/tests/unit_tests/tracers/test_base_tracer.py#L193-L251)
- Contract: serialized `Run` records whose `run_type` is `tool`, including the
  upstream structured-input case.
- License: MIT.

## PydanticAI

- Repository: <https://github.com/pydantic/pydantic-ai>
- Commit: `d995cfee9fa4243e3a6f5d8e6762b841f7fde839`
- Sources:
  [`tests/test_logfire.py`](https://github.com/pydantic/pydantic-ai/blob/d995cfee9fa4243e3a6f5d8e6762b841f7fde839/tests/test_logfire.py#L54-L75)
  and
  [`pydantic_ai_slim/pydantic_ai/capabilities/instrumentation.py`](https://github.com/pydantic/pydantic-ai/blob/d995cfee9fa4243e3a6f5d8e6762b841f7fde839/pydantic_ai_slim/pydantic_ai/capabilities/instrumentation.py#L399-L424)
- Contract: Logfire `exported_spans_as_dict()` records with standard
  `gen_ai.operation.name`, tool name, call ID, arguments, and result attributes.
- Source kind: adapted exporter snapshot. OTel nanosecond timestamps and numeric
  test context IDs match the pinned upstream snapshot; generated values are
  deterministic replacements.
- License: MIT.
