mod pre_tool_use;
mod shared;
mod stop;
mod version;

use std::collections::HashMap;

use clap::{Args, Parser, Subcommand};

use crate::shared::agent::Agent;

#[derive(Debug, Parser)]
#[command(version = version::clap_version())]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Args)]
struct EventArgs {
    #[arg(long)]
    agent: Agent,
}

#[derive(Debug, Subcommand)]
enum Commands {
    PreToolUse(EventArgs),
    Stop,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::PreToolUse(args) => match pre_tool_use::run(args.agent) {
            Ok(Some(output)) => println!("{output}"),
            Ok(None) => {}
            Err(err) => {
                let reason = err.to_string();
                let attributes = pre_tool_use::log_attributes(&err);
                let log_result = log_pre_tool_use_error(attributes, &reason);
                let log_error = log_result.err();
                if let Some(log_err) = log_error {
                    eprintln!("logging failed: {log_err}");
                }
                eprintln!("{reason}");
            }
        },
        Commands::Stop => match stop::run() {
            Ok(()) => {}
            Err(reason) => match shared::response::stop_block(reason) {
                Ok(output) => println!("{output}"),
                Err(err) => {
                    let error_reason = err.to_string();
                    let log_result = log_stop_error(&error_reason);
                    let log_error = log_result.err();
                    if let Some(log_err) = log_error {
                        eprintln!("logging failed: {log_err}");
                    }
                    eprintln!("{error_reason}");
                }
            },
        },
    }
}

fn log_pre_tool_use_error(
    mut attributes: HashMap<String, String>,
    reason: &str,
) -> Result<(), shared::logger::Error> {
    let key = String::from("reason");
    let value = reason.to_owned();
    attributes.insert(key, value);
    log_command_error("PreToolUse", attributes)
}

fn log_stop_error(reason: &str) -> Result<(), shared::logger::Error> {
    let key = String::from("reason");
    let value = reason.to_owned();
    let attributes = HashMap::from([(key, value)]);
    log_command_error("Stop", attributes)
}

fn log_command_error(
    command_name: &str,
    attributes: HashMap<String, String>,
) -> Result<(), shared::logger::Error> {
    shared::logger::error(command_name, attributes)
}
