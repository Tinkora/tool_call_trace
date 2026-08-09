use thiserror::Error;

/// Stable error type for the tool-call trace contract.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    /// Generic JSON parse failure (invalid JSON syntax).
    #[error("JSON parse error: {0}")]
    ParseError(String),

    /// Recognized the format but the structure is invalid (missing fields, wrong types).
    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    /// No tool calls found in the provided log.
    #[error("No tool calls found in the log")]
    EmptyLog,

    /// Internal analysis computation failure.
    #[error("Analysis error: {0}")]
    AnalysisError(String),
}

impl CoreError {
    /// Returns a stable machine error code for Rust and WebAssembly consumers.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ParseError(_) => "PARSE_ERROR",
            Self::InvalidFormat(_) => "INVALID_FORMAT",
            Self::EmptyLog => "EMPTY_LOG",
            Self::AnalysisError(_) => "ANALYSIS_ERROR",
        }
    }
}
