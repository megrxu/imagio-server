use std::{io::Write, path::Path, str::FromStr};

use chrono::Utc;
use clap::{Parser, Subcommand};
use image::{io::Reader as ImageReader, DynamicImage};
use mime_guess::Mime;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, MutexGuard, RwLock};

use crate::{variant::Variant, ImagioError};

#[derive(Debug)]
pub(crate) struct ImagioState {
    pub(crate) db: RwLock<Mutex<Connection>>,
    pub(crate) token: String,
    pub(crate) store: String,
    pub(crate) cache: String,
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
    #[clap(short, default_value = "data/images")]
    pub(crate) store: String,
    #[clap(short, default_value = "data/cache")]
    pub(crate) cache: String,
    #[clap(short, default_value = "pBxTJTxHRtQetTGf")]
    pub(crate) token: String,
    #[clap(subcommand)]
    pub(crate) command: ImagioCommand,
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
                var.to_string(),
                self.ext()
            ),
        }
    }

    pub(crate) fn store<T, P>(&self, buf: &[u8], path: T, filename: P) -> Result<(), ImagioError>
    where
        T: AsRef<Path>,
        P: AsRef<Path>,
    {
        let write_path = path.as_ref().join(filename);
        let mut file = std::fs::File::create(&write_path)?;
        file.write_all(&buf)?;
        tracing::info!("Image saved to: {:?}", write_path);
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
    pub(crate) fn new(cli: ImagioCli) -> Self {
        let db = Connection::open(&cli.db).unwrap();
        let db = RwLock::new(Mutex::new(db));
        ImagioState {
            db,
            token: cli.token,
            store: cli.store,
            cache: cli.cache,
        }
    }

    pub(crate) async fn get(&self, uuid: &str) -> Result<ImagioImage, ImagioError> {
        let lock = self.db.read().await;
        let conn = &lock.lock().await;
        let mut stmt = conn.prepare("SELECT uuid, category, mime FROM images WHERE uuid = ?")?;
        let mut rows = stmt.query(&[&uuid])?;

        while let Some(row) = rows.next()? {
            let image = ImagioImage::try_from(row)?;
            return Ok(image);
        }
        return Err(ImagioError::NotFound);
    }

    pub(crate) async fn put(&self, image: &ImagioImage) -> Result<(), ImagioError> {
        let lock = self.db.write().await;
        let conn = &lock.lock().await;
        let mut stmt = conn.prepare(
            "INSERT INTO images (uuid, category, mime, create_time) VALUES (?, ?, ?, ?)",
        )?;
        let _ = stmt.execute(&[
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
            let mut rows = stmt.query(&[&uuid])?;

            if let Some(row) = rows.next()? {
                ImagioImage::try_from(row)?
            } else {
                return Err(ImagioError::NotFound);
            }
        };

        // Delete the image from the store
        let path: &Path = self.store.as_ref();
        let path = path.join(image.filename(&Variant::Original));
        std::fs::remove_file(&path)?;
        tracing::info!("Image deleted from: {:?}", path);

        // Delete the image from the database
        {
            let lock = self.db.write().await;
            let conn = &lock.lock().await;
            let mut stmt = conn.prepare("DELETE FROM images WHERE uuid = ?")?;
            let _ = stmt.execute(&[&uuid])?;
        }

        Ok(image)
    }

    pub(crate) async fn list(
        &self,
        limit: usize,
        skip: usize,
    ) -> Result<Vec<ImagioImage>, ImagioError> {
        let lock = self.db.read().await;
        let conn = &lock.lock().await;
        let mut stmt = conn.prepare(
            "SELECT uuid, category, mime FROM images ORDER BY create_time DESC LIMIT ? OFFSET ? ",
        )?;
        let mut rows = stmt.query(&[&(limit as i64), &(skip as i64)])?;

        let mut images = Vec::new();
        while let Some(row) = rows.next()? {
            let image = ImagioImage::try_from(row)?;
            images.push(image);
        }

        Ok(images)
    }
}
