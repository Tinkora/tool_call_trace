use serde::Serialize;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process::ExitCode;
use tool_call_trace_core::parse::MAX_INPUT_BYTES;
use tool_call_trace_core::{
    ArgumentDiagnostic, CoreError, RedactionConfig, RedactionOutcome, ToolCallLog,
    find_retry_loop_findings, parse_agent_trace, parse_generic_array, parse_langchain_format,
    parse_openai_agents_format, parse_openai_format, parse_pydantic_ai_logfire_format,
    parse_tool_inventory, redact_log, validate_tool_arguments,
};

const USAGE: &str = "Usage: tool-call-trace check [--format FORMAT] [--tools FILE] [--redact] [--redact-path POINTER] [FILE|-]\n\nFormats: auto, generic, openai-run-steps, openai-agents, langchain, pydantic-ai";
const MAX_TEXT_DIAGNOSTICS: usize = 20;

#[derive(Clone, Copy)]
enum TraceFormat {
    Auto,
    Generic,
    OpenAIRunSteps,
    OpenAIAgents,
    LangChain,
    PydanticAI,
}

impl TraceFormat {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "auto" => Ok(Self::Auto),
            "generic" => Ok(Self::Generic),
            "openai-run-steps" => Ok(Self::OpenAIRunSteps),
            "openai-agents" => Ok(Self::OpenAIAgents),
            "langchain" => Ok(Self::LangChain),
            "pydantic-ai" => Ok(Self::PydanticAI),
            _ => Err(CliError::usage("unsupported --format value")),
        }
    }
}

struct Options {
    format: TraceFormat,
    redact: bool,
    redaction_paths: Vec<String>,
    tools: Option<String>,
    input: String,
}

struct CliError {
    message: String,
    code: u8,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: 2,
        }
    }

    fn contract(error: CoreError) -> Self {
        Self {
            message: format!("{}: {error}", error.code()),
            code: 1,
        }
    }

    fn redacted_contract(error: CoreError) -> Self {
        Self {
            message: format!(
                "{}: input could not be parsed or redacted while redaction was enabled",
                error.code()
            ),
            code: 1,
        }
    }

    fn input(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: 1,
        }
    }
}

#[derive(Serialize)]
struct ContractReport {
    valid: bool,
    redacted_values: u32,
    retry_loop_findings: Vec<tool_call_trace_core::RetryLoopFinding>,
    argument_diagnostics: Vec<ArgumentDiagnostic>,
    log: ToolCallLog,
}

fn parse_options() -> Result<Option<Options>, CliError> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        return Err(CliError::usage("missing check command"));
    };
    if matches!(command.as_str(), "-h" | "--help") {
        return Ok(None);
    }
    if command != "check" {
        return Err(CliError::usage("the only supported command is check"));
    }

    let mut format = TraceFormat::Auto;
    let mut redact = false;
    let mut redaction_paths = Vec::new();
    let mut tools = None;
    let mut input = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => return Ok(None),
            "--format" => {
                let value = args
                    .next()
                    .ok_or_else(|| CliError::usage("--format requires a value"))?;
                format = TraceFormat::parse(&value)?;
            }
            "--redact" => redact = true,
            "--tools" => {
                let value = args
                    .next()
                    .ok_or_else(|| CliError::usage("--tools requires a file path"))?;
                if value == "-" {
                    return Err(CliError::usage("--tools must be a file, not stdin"));
                }
                if tools.replace(value).is_some() {
                    return Err(CliError::usage("--tools may only be specified once"));
                }
            }
            "--redact-path" => {
                redaction_paths.push(
                    args.next()
                        .ok_or_else(|| CliError::usage("--redact-path requires a value"))?,
                );
            }
            _ if argument.starts_with("--format=") => {
                format = TraceFormat::parse(&argument["--format=".len()..])?;
            }
            "-" => {
                if input.replace(argument).is_some() {
                    return Err(CliError::usage("only one input file is allowed"));
                }
            }
            _ if argument.starts_with('-') => {
                return Err(CliError::usage("unknown option"));
            }
            _ => {
                if input.replace(argument).is_some() {
                    return Err(CliError::usage("only one input file is allowed"));
                }
            }
        }
    }
    if !redact && !redaction_paths.is_empty() {
        return Err(CliError::usage("--redact-path requires --redact"));
    }

    Ok(Some(Options {
        format,
        redact,
        redaction_paths,
        tools,
        input: input.unwrap_or_else(|| "-".into()),
    }))
}

fn read_bounded(mut reader: impl Read) -> Result<String, CliError> {
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| CliError::input("INPUT_ERROR: unable to read input"))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(CliError::input(format!(
            "INVALID_FORMAT: input exceeds the {MAX_INPUT_BYTES}-byte limit"
        )));
    }
    String::from_utf8(bytes).map_err(|_| CliError::input("INVALID_FORMAT: input must be UTF-8"))
}

fn read_input(path: &str) -> Result<String, CliError> {
    if path == "-" {
        read_bounded(io::stdin().lock())
    } else {
        let file = File::open(path)
            .map_err(|_| CliError::input("INPUT_ERROR: unable to open input file"))?;
        read_bounded(file)
    }
}

fn parse_trace(format: TraceFormat, input: &str) -> Result<ToolCallLog, CoreError> {
    match format {
        TraceFormat::Auto => parse_agent_trace(input),
        TraceFormat::Generic => parse_generic_array(input),
        TraceFormat::OpenAIRunSteps => parse_openai_format(input),
        TraceFormat::OpenAIAgents => parse_openai_agents_format(input),
        TraceFormat::LangChain => parse_langchain_format(input),
        TraceFormat::PydanticAI => parse_pydantic_ai_logfire_format(input),
    }
}

fn execute(options: Options) -> Result<(), CliError> {
    let input = read_input(&options.input)?;
    let log = parse_trace(options.format, &input).map_err(|error| {
        if options.redact {
            CliError::redacted_contract(error)
        } else {
            CliError::contract(error)
        }
    })?;
    let retry_loop_findings = find_retry_loop_findings(&log);
    let argument_diagnostics = if let Some(path) = options.tools.as_deref() {
        let inventory = read_input(path)?;
        let tools = parse_tool_inventory(&inventory).map_err(CliError::contract)?;
        validate_tool_arguments(&log, &tools)
    } else {
        Vec::new()
    };
    let RedactionOutcome {
        log,
        redacted_values,
    } = if options.redact {
        redact_log(
            &log,
            &RedactionConfig {
                paths: options.redaction_paths,
            },
        )
        .map_err(CliError::contract)?
    } else {
        RedactionOutcome {
            log,
            redacted_values: 0,
        }
    };
    let total_calls = log.total_calls;
    let report = ContractReport {
        valid: argument_diagnostics.is_empty(),
        redacted_values,
        retry_loop_findings,
        argument_diagnostics: argument_diagnostics.clone(),
        log,
    };

    serde_json::to_writer(io::stdout().lock(), &report)
        .map_err(|_| CliError::input("OUTPUT_ERROR: unable to write JSON"))?;
    println!();
    if argument_diagnostics.is_empty() {
        eprintln!(
            "valid: {total_calls} tool call{}, {redacted_values} value{} redacted",
            if total_calls == 1 { "" } else { "s" },
            if redacted_values == 1 { "" } else { "s" }
        );
        Ok(())
    } else {
        for diagnostic in argument_diagnostics.iter().take(MAX_TEXT_DIAGNOSTICS) {
            eprintln!(
                "{}: {} [{}]",
                diagnostic.code.as_str(),
                text_label(&diagnostic.message),
                diagnostic
                    .call_ids
                    .iter()
                    .map(|id| text_label(id))
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        if argument_diagnostics.len() > MAX_TEXT_DIAGNOSTICS {
            eprintln!(
                "... {} additional argument diagnostics omitted from text output",
                argument_diagnostics.len() - MAX_TEXT_DIAGNOSTICS
            );
        }
        Err(CliError::input(format!(
            "argument validation failed with {} diagnostic{}",
            argument_diagnostics.len(),
            if argument_diagnostics.len() == 1 {
                ""
            } else {
                "s"
            }
        )))
    }
}

fn text_label(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(160)
        .collect()
}

fn main() -> ExitCode {
    match parse_options() {
        Ok(None) => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Some(options)) => match execute(options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                let _ = writeln!(io::stderr().lock(), "{}", error.message);
                ExitCode::from(error.code)
            }
        },
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "{}\n\n{USAGE}", error.message);
            ExitCode::from(error.code)
        }
    }
}
