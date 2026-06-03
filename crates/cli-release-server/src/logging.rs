//! Logging configuration for the release-server binary.

use std::error::Error;

use clap::{Args, ValueEnum};
use tracing_subscriber::EnvFilter;

/// Logging CLI and environment arguments.
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
pub struct LoggingArgs {
    /// Log level.
    #[arg(long, env = "CLI_RELEASE_LOG_LEVEL", global = true)]
    pub log_level: Option<LogLevel>,
    /// Log output format.
    #[arg(long, env = "CLI_RELEASE_LOG_FORMAT", global = true)]
    pub log_format: Option<LogFormat>,
}

/// Supported log levels.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum LogLevel {
    /// Error events only.
    Error,
    /// Warning and error events.
    Warn,
    /// Informational events.
    Info,
    /// Debug events.
    Debug,
    /// Trace events.
    Trace,
}

/// Supported log formats.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum LogFormat {
    /// Human-readable pretty output.
    Pretty,
    /// Compact human-readable output.
    Compact,
    /// JSON lines.
    Json,
}

/// Installs the process-wide tracing subscriber.
pub fn init(args: &LoggingArgs) -> Result<(), Box<dyn Error + Send + Sync>> {
    let level = args.log_level.unwrap_or(LogLevel::Info).as_str();
    let filter = EnvFilter::new(level);
    match args.log_format.unwrap_or(LogFormat::Pretty) {
        LogFormat::Pretty => tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(filter)
            .try_init(),
        LogFormat::Compact => tracing_subscriber::fmt()
            .compact()
            .with_writer(std::io::stderr)
            .with_env_filter(filter)
            .try_init(),
        LogFormat::Json => tracing_subscriber::fmt()
            .json()
            .with_writer(std::io::stderr)
            .with_env_filter(filter)
            .try_init(),
    }
}

impl LogLevel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}
