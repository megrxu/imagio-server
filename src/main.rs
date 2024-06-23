mod api;
mod app;
mod error;
mod server;
mod variant;

use app::*;
use clap::Parser;
use error::*;
use server::*;
use variant::generate;

#[tokio::main]
async fn main() -> Result<(), ImagioError> {
    let cli = ImagioCli::parse();

    tracing_subscriber::fmt::fmt().init();

    match cli.command {
        ImagioCommand::Init { .. } => {
            tracing::info!("Initializing database");
        }
        ImagioCommand::Generate => {
            tracing::info!("Generating variants");
            generate()?;
        }
        ImagioCommand::Serve => {
            let state = ImagioState::new(cli)?;
            let async_state = std::sync::Arc::new(state);
            tracing::info!("Starting server at {}", async_state.bind);
            server(async_state).await?;
        }
    }

    Ok(())
}
