use clap::{Parser, Subcommand};
use mime_guess::Mime;
use rusqlite::Connection;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug)]
pub(crate) struct ImagioState {
    pub(crate) db: RwLock<Mutex<Connection>>,
    pub(crate) token: String,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ImagioCommand {
    Init {
        #[clap(short, long, default_value = "false")]
        force: bool,
    },
    Generate,
    Serve,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct ImagioCli {
    #[clap(short, default_value = "data/imagio.db")]
    pub(crate) db: String,
    #[clap(short, default_value = "pBxTJTxHRtQetTGf")]
    pub(crate) token: String,
    #[clap(subcommand)]
    pub(crate) command: ImagioCommand,
}

#[derive(Debug)]
pub struct ImagioImage {
    pub(crate) uuid: String,
    pub(crate) category: String,
    pub(crate) mime: Mime,
}
