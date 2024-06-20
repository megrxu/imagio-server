mod app;
mod db;
mod error;
mod server;
mod variant;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use app::*;
use clap::Parser;
use db::*;
use error::*;
use rusqlite::Connection;
use server::*;
use variant::generate;

#[tokio::main]
async fn main() -> Result<(), ImagioError> {
    let cli = ImagioCli::parse();

    tracing_subscriber::fmt::fmt().init();

    match cli.command {
        ImagioCommand::Init { force } => {
            tracing::info!("Initializing database");
            init_db(&cli.db, force)?;
            refresh(&cli.db)?;
        }
        ImagioCommand::Generate => {
            tracing::info!("Generating variants");
            generate()?;
        }
        ImagioCommand::Serve => {
            let state = ImagioState {
                db: RwLock::new(Mutex::new(Connection::open(&cli.db)?)),
                token: cli.token,
            };
            let async_state = Arc::new(state);
            tracing::info!("Starting server at http://localhost:4000");
            server(async_state).await?;
        }
    }

    Ok(())
}
