mod pre_tool_use;
mod prompt_submit;
mod shared;
mod stop;

use std::collections::HashMap;

use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    PreToolUse,
    PromptSubmit,
    Stop,
}

fn run_command(command: &Commands) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match command {
        Commands::PreToolUse => Ok(pre_tool_use::run()?),
        Commands::PromptSubmit => {
            prompt_submit::run()?;
            Ok(None)
        }
        Commands::Stop => Ok(stop::run()?),
    }
}

fn main() {
    let cli = Cli::parse();

    match run_command(&cli.command) {
        Ok(Some(output)) => println!("{output}"),
        Ok(None) => {}
        Err(err) => {
            let command_name = format!("{:?}", cli.command);
            if let Err(log_err) = shared::logger::error(
                &command_name,
                HashMap::from([("reason".to_owned(), err.to_string())]),
            ) {
                eprintln!("logging failed: {log_err}");
            }
            eprintln!("{err}");
        }
    }
}
