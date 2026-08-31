//! Output discipline: stdout carries data, stderr carries everything else.
//!
//! Format defaults to `json` when stdout is not a terminal and `table` when it
//! is, so piping into `jq` works without a flag and a human at a prompt gets
//! something readable.

use serde::Serialize;
use std::io::{IsTerminal, Write};

/// Exit codes. Stable across versions — changing one is a breaking change.
pub mod exit {
    /// Completed successfully.
    pub const OK: i32 = 0;
    /// Something went wrong; read stderr.
    pub const GENERAL: i32 = 1;
    /// Bad arguments or an invalid template.
    pub const USAGE: i32 = 2;
    /// A recording id is not in the FindOpera database.
    pub const NOT_FOUND: i32 = 3;
    /// The API was unreachable or errored; worth retrying.
    pub const API: i32 = 6;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub enum Format {
    /// Machine-readable JSON (default when stdout is piped).
    Json,
    /// One rendered value per line (default at a terminal).
    Text,
    /// One JSON object per line, for streaming.
    Ndjson,
}

impl Format {
    /// Resolve `--format` against TTY detection.
    pub fn resolve(explicit: Option<Format>) -> Format {
        explicit.unwrap_or(if std::io::stdout().is_terminal() {
            Format::Text
        } else {
            Format::Json
        })
    }
}

pub fn print_json<T: Serialize>(value: &T) {
    let mut stdout = std::io::stdout().lock();
    match serde_json::to_string_pretty(value) {
        Ok(s) => {
            let _ = writeln!(stdout, "{s}");
        }
        Err(e) => eprintln!("findopera: could not serialize output: {e}"),
    }
}

pub fn print_ndjson<T: Serialize>(values: &[T]) {
    let mut stdout = std::io::stdout().lock();
    for v in values {
        if let Ok(s) = serde_json::to_string(v) {
            let _ = writeln!(stdout, "{s}");
        }
    }
}

/// A machine-readable failure. Printed as JSON to stderr in non-TTY mode and as
/// prose at a terminal; either way it carries a code, a suggestion, and whether
/// retrying could help.
#[derive(Serialize)]
pub struct Failure {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub details: Vec<String>,
}

impl Failure {
    pub fn new(error: &str, message: impl Into<String>) -> Self {
        Failure {
            error: error.to_string(),
            message: message.into(),
            suggestion: None,
            retryable: false,
            input: None,
            details: Vec::new(),
        }
    }
    pub fn suggest(mut self, s: impl Into<String>) -> Self {
        self.suggestion = Some(s.into());
        self
    }
    pub fn retryable(mut self, r: bool) -> Self {
        self.retryable = r;
        self
    }
    pub fn input(mut self, i: impl Into<String>) -> Self {
        self.input = Some(i.into());
        self
    }
    pub fn details(mut self, d: Vec<String>) -> Self {
        self.details = d;
        self
    }

    /// Emit to stderr and return the process exit code.
    pub fn emit(self, code: i32) -> i32 {
        if std::io::stderr().is_terminal() {
            eprintln!("findopera: {}", self.message);
            for d in &self.details {
                eprintln!("  {d}");
            }
            if let Some(input) = &self.input {
                eprintln!("  input: {input}");
            }
            if let Some(s) = &self.suggestion {
                eprintln!("\n{s}");
            }
        } else {
            match serde_json::to_string_pretty(&self) {
                Ok(s) => eprintln!("{s}"),
                Err(_) => eprintln!("findopera: {}", self.message),
            }
        }
        code
    }
}
