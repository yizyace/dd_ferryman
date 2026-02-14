use anyhow::Result;
use clap::Parser;

use dd_ferryman::cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { foreground: _ } => {
            println!("start: not yet implemented");
        }
        Commands::Stop => {
            println!("stop: not yet implemented");
        }
        Commands::Status => {
            println!("status: not yet implemented");
        }
    }

    Ok(())
}
