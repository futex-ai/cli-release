use clap::{CommandFactory, Parser};

use cli_release_server::logging::LogFormat;

use crate::Cli;

#[test]
fn parses_logging_flags_after_subcommand() {
    let cli = Cli::try_parse_from(["cli-release-server", "serve", "--log-format", "json"])
        .expect("parse CLI");

    assert_eq!(cli.logging.log_format, Some(LogFormat::Json));
}

#[test]
fn help_mentions_logging_env_vars() {
    let help = Cli::command().render_long_help().to_string();

    assert!(help.contains("CLI_RELEASE_LOG_LEVEL"));
    assert!(help.contains("CLI_RELEASE_LOG_FORMAT"));
}
