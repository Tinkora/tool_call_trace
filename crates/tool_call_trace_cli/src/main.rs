use serde::Serialize;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process::ExitCode;
use tool_call_trace_core::parse::MAX_INPUT_BYTES;
use tool_call_trace_core::{
    CoreError, RedactionConfig, RedactionOutcome, ToolCallLog, parse_agent_trace,
    parse_generic_array, parse_langchain_format, parse_openai_agents_format, parse_openai_format,
    parse_pydantic_ai_logfire_format, redact_log,
};

const USAGE: &str = "Usage: tool-call-trace check [--format FORMAT] [--redact] [--redact-path POINTER] [FILE|-]\n\nFormats: auto, generic, openai-run-steps, openai-agents, langchain, pydantic-ai";

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
        valid: true,
        redacted_values,
        log,
    };

    serde_json::to_writer(io::stdout().lock(), &report)
        .map_err(|_| CliError::input("OUTPUT_ERROR: unable to write JSON"))?;
    println!();
    eprintln!(
        "valid: {total_calls} tool call{}, {redacted_values} value{} redacted",
        if total_calls == 1 { "" } else { "s" },
        if redacted_values == 1 { "" } else { "s" }
    );
    Ok(())
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
