//! `cli-release-server` entry point.

use std::net::SocketAddr;

use clap::{Args, Parser, Subcommand};
use cli_release_server::{
    config::{BIND_ENV, ServerConfig},
    logging::LoggingArgs,
};
use thiserror::Error;

#[derive(Parser, Debug)]
#[command(name = "cli-release-server")]
struct Cli {
    #[command(flatten)]
    logging: LoggingArgs,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Serve(ServeArgs),
}

#[derive(Args, Debug)]
struct ServeArgs {
    #[arg(long)]
    bind: Option<String>,
}

#[derive(Debug, Error)]
enum Error {
    #[error("[cli_release_server/main] invalid configuration: {source}")]
    Config { source: cli_release_server::Error },
    #[error("[cli_release_server/main] failed to parse bind `{value}`: {source}")]
    ParseBind {
        value: String,
        source: std::net::AddrParseError,
    },
    #[error("[cli_release_server/main] failed to bind `{value}`: {source}")]
    Bind {
        value: SocketAddr,
        source: std::io::Error,
    },
    #[error("[cli_release_server/main] HTTP server failed: {source}")]
    Serve { source: std::io::Error },
}

type Result<T> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let _ = cli_release_server::logging::init(&cli.logging);
    if let Err(error) = run(cli).await {
        tracing::error!(error = ?error, "top-level error");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Serve(args) => serve(args).await,
    }
}

async fn serve(args: ServeArgs) -> Result<()> {
    let config = ServerConfig::from_env().map_err(|source| Error::Config { source })?;
    let bind = args
        .bind
        .or_else(|| {
            std::env::var(BIND_ENV)
                .ok()
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| config.bind.clone());
    let bind_addr: SocketAddr = bind.parse().map_err(|source| Error::ParseBind {
        value: bind,
        source,
    })?;
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .map_err(|source| Error::Bind {
            value: bind_addr,
            source,
        })?;
    tracing::info!(%bind_addr, "starting release server");
    let app = cli_release_server::routes::app(config).map_err(|source| Error::Config { source })?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|source| Error::Serve { source })
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
#[path = "_tests_/main_tests.rs"]
mod main_tests;
