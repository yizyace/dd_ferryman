use anyhow::Result;
use clap::Parser;

use dd_ferryman::cli::{Cli, Commands};
use dd_ferryman::commands;

fn setup_logging(foreground: bool) {
    use tracing_subscriber::EnvFilter;

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("dd_ferryman=info"));

    if foreground {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .init();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { foreground } => {
            setup_logging(foreground);
            commands::start::run(foreground).await?;
        }
        Commands::Stop => {
            setup_logging(false);
            commands::stop::run()?;
        }
        Commands::Status => {
            setup_logging(false);
            commands::status::run()?;
        }
        Commands::Install => {
            commands::install::run()?;
        }
        Commands::Uninstall => {
            commands::uninstall::run()?;
        }
    }

    Ok(())
}
