mod hook_response;
mod stop;

use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Stop,
}

fn run_command(command: &Commands) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match command {
        Commands::Stop => Ok(stop::run()?),
    }
}

fn main() {
    let cli = Cli::parse();

    match run_command(&cli.command) {
        Ok(Some(output)) => println!("{output}"),
        Ok(None) => {}
        Err(err) => eprintln!("{err}"),
    }
}
