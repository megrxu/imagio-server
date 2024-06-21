use std::{path::Path, str::FromStr};

use chrono::Utc;
use clap::{Parser, Subcommand};
use mime_guess::Mime;
use rusqlite::{Connection};
use serde::Serialize;
use tokio::sync::{Mutex, RwLock};

use crate::{variant::Variant, ImagioError};
use opendal::{services::Fs, Operator};

#[derive(Debug)]
pub(crate) struct ImagioState {
    pub(crate) db: RwLock<Mutex<Connection>>,
    pub(crate) slug: String,
    pub(crate) storage: ImagioStorageOperator,
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

#[derive(Parser, Debug, Clone, clap::ValueEnum)]
pub(crate) enum ImagioStorageBackend {
    Fs,
    S3,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct ImagioStorageParameters {
    #[clap(short, default_value = "data/cache")]
    pub(crate) cache: String,
    #[clap(short, default_value = "data/images")]
    pub(crate) store: String,
    #[clap(long, default_value = None)]
    pub(crate) bucket: Option<String>,
    #[clap(long, default_value = None)]
    pub(crate) region: Option<String>,
    #[clap(long, default_value = None)]
    pub(crate) endpoint: Option<String>,
    #[clap(long, default_value = None)]
    pub(crate) access_key_id: Option<String>,
    #[clap(long, default_value = None)]
    pub(crate) secret_access_key: Option<String>,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct ImagioStorage {
    pub(crate) backend: ImagioStorageBackend,
    #[clap(flatten)]
    pub(crate) parameters: ImagioStorageParameters,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct ImagioCli {
    #[clap(short, default_value = "data/imagio.db")]
    pub(crate) db: String,
    #[clap(flatten)]
    pub(crate) storage: ImagioStorage,
    #[clap(long, default_value = "pBxTJTxHRtQetTGf")]
    pub(crate) account_id: String,
    #[clap(subcommand)]
    pub(crate) command: ImagioCommand,
}

#[derive(Debug, Clone)]
pub struct ImagioStorageOperator {
    pub(crate) cache: Operator,
    pub(crate) store: Operator,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImagioImage {
    pub(crate) uuid: String,
    pub(crate) category: String,
    #[serde(skip)]
    pub(crate) mime: Mime,
}

impl ImagioImage {
    pub(crate) fn new(uuid: &str, category: &str, mime: &str) -> Result<Self, ImagioError> {
        let mime = Mime::from_str(mime)?;
        Ok(ImagioImage {
            uuid: uuid.to_string(),
            category: category.to_string(),
            mime,
        })
    }

    pub(crate) fn ext(&self) -> String {
        self.mime.subtype().to_string().to_ascii_uppercase()
    }

    pub(crate) fn filename(&self, variant: &Variant) -> String {
        match variant {
            Variant::Original => format!("{}/{}.{}", self.category, self.uuid, self.ext()),
            var => format!(
                "{}_{}_{}.{}",
                self.category,
                self.uuid,
                var,
                self.ext()
            ),
        }
    }

    pub(crate) async fn store<T: Into<opendal::Buffer>>(
        &self,
        buf: T,
        op: Operator,
        filename: &str,
    ) -> Result<(), ImagioError> {
        op.write(filename, buf).await?;
        tracing::info!("Image saved to: {:?}", &filename);
        Ok(())
    }
}

impl TryFrom<&rusqlite::Row<'_>> for ImagioImage {
    type Error = ImagioError;

    fn try_from(row: &rusqlite::Row) -> Result<Self, Self::Error> {
        let uuid: String = row.get(0)?;
        let category: String = row.get(1)?;
        let mime: String = row.get(2)?;
        let image = ImagioImage {
            uuid: uuid.to_string(),
            category,
            mime: Mime::from_str(&mime)?,
        };
        Ok(image)
    }
}

impl ImagioState {
    pub(crate) fn new(cli: ImagioCli) -> Result<Self, ImagioError> {
        let db = Connection::open(&cli.db).unwrap();
        let db = RwLock::new(Mutex::new(db));

        let storage = match &cli.storage.backend {
            ImagioStorageBackend::Fs => {
                let absulute = |p: &str| -> Result<String, ImagioError> {
                    let path = std::path::absolute(Path::new(p))?;
                    Ok(path.to_str().unwrap().to_string())
                };
                let mut store_builder = Fs::default();
                let path = absulute(&cli.storage.parameters.store)?;
                store_builder.root(&path);
                let store = Operator::new(store_builder)?.finish();

                let mut cache_builder = Fs::default();
                let path = absulute(&cli.storage.parameters.cache)?;
                cache_builder.root(&path);
                let cache = Operator::new(cache_builder)?.finish();
                ImagioStorageOperator { cache, store }
            }
            ImagioStorageBackend::S3 => {
                let mut store_builder = opendal::services::S3::default();
                store_builder.region(&cli.storage.parameters.region.clone().unwrap());
                store_builder.bucket(&cli.storage.parameters.bucket.clone().unwrap());
                store_builder.endpoint(&cli.storage.parameters.endpoint.clone().unwrap());
                store_builder.access_key_id(&cli.storage.parameters.access_key_id.clone().unwrap());
                store_builder
                    .secret_access_key(&cli.storage.parameters.secret_access_key.clone().unwrap());
                store_builder.root(&cli.storage.parameters.store);
                let store = Operator::new(store_builder)?.finish();

                let mut cache_builder = opendal::services::S3::default();
                cache_builder.region(&cli.storage.parameters.region.unwrap());
                cache_builder.bucket(&cli.storage.parameters.bucket.unwrap());
                cache_builder.endpoint(&cli.storage.parameters.endpoint.clone().unwrap());
                cache_builder.access_key_id(&cli.storage.parameters.access_key_id.clone().unwrap());
                cache_builder
                    .secret_access_key(&cli.storage.parameters.secret_access_key.clone().unwrap());
                cache_builder.root(&cli.storage.parameters.cache);
                let cache = Operator::new(cache_builder)?.finish();
                ImagioStorageOperator { cache, store }
            }
        };

        Ok(ImagioState {
            db,
            slug: cli.account_id,
            storage,
        })
    }

    pub(crate) async fn get(&self, uuid: &str) -> Result<ImagioImage, ImagioError> {
        let lock = self.db.read().await;
        let conn = &lock.lock().await;
        let mut stmt = conn.prepare("SELECT uuid, category, mime FROM images WHERE uuid = ?")?;
        let mut rows = stmt.query([&uuid])?;

        if let Some(row) = rows.next()? {
            let image = ImagioImage::try_from(row)?;
            return Ok(image);
        }
        Err(ImagioError::NotFound)
    }

    pub(crate) async fn put(&self, image: &ImagioImage) -> Result<(), ImagioError> {
        let lock = self.db.write().await;
        let conn = &lock.lock().await;
        let mut stmt = conn.prepare(
            "INSERT INTO images (uuid, category, mime, create_time) VALUES (?, ?, ?, ?)",
        )?;
        let _ = stmt.execute([
            &image.uuid,
            &image.category,
            &image.mime.to_string(),
            &Utc::now().to_string(),
        ])?;
        Ok(())
    }

    pub(crate) async fn delete(&self, uuid: &str) -> Result<ImagioImage, ImagioError> {
        let image = {
            let lock = self.db.read().await;
            let conn = &lock.lock().await;
            let mut stmt =
                conn.prepare("SELECT uuid, category, mime FROM images WHERE uuid = ?")?;
            let mut rows = stmt.query([&uuid])?;

            if let Some(row) = rows.next()? {
                ImagioImage::try_from(row)?
            } else {
                return Err(ImagioError::NotFound);
            }
        };

        // Delete the image from the store
        let filename = image.filename(&Variant::Original);
        self.storage.store.delete(&filename).await?;
        tracing::info!("Image deleted from: {:?} (Store)", filename);

        // Delete the image from the database
        {
            let lock = self.db.write().await;
            let conn = &lock.lock().await;
            let mut stmt = conn.prepare("DELETE FROM images WHERE uuid = ?")?;
            let _ = stmt.execute([&uuid])?;
        }

        Ok(image)
    }

    pub(crate) async fn list(
        &self,
        category: String,
        limit: usize,
        skip: usize,
    ) -> Result<Vec<ImagioImage>, ImagioError> {
        let lock = self.db.read().await;
        let conn = &lock.lock().await;
        let mut stmt = conn.prepare(
            "SELECT uuid, category, mime FROM images WHERE category = ? ORDER BY create_time DESC LIMIT ? OFFSET ?",
        )?;
        let mut rows = stmt.query([
            category,
            (limit as i64).to_string(),
            (skip as i64).to_string(),
        ])?;

        let mut images = Vec::new();
        while let Some(row) = rows.next()? {
            let image = ImagioImage::try_from(row)?;
            images.push(image);
        }

        Ok(images)
    }
}
