use rusqlite::Connection;

use crate::{ImagioImage, ImagioError};

pub fn init_db(path: &str, force: bool) -> rusqlite::Result<()> {
    if !force {
        if std::path::Path::new(path).exists() {
            tracing::warn!("Database already exists, skipping init");
            return Ok(());
        }
    }
    let conn = Connection::open(path)?;
    let sql = include_str!("../schema.sql");
    println!("{}", sql);
    conn.execute(sql, ())?;
    conn.close().ok();
    Ok(())
}

pub fn refresh(db_path: &str) -> Result<(), ImagioError> {
    let conn = Connection::open(db_path)?;
    // list all categories
    let categories = std::fs::read_dir("data/images")?;

    // for each category, list all images
    for category in categories {
        let category = category?;
        let category_name = category.file_name().into_string().unwrap();
        let images = std::fs::read_dir(category.path())?;

        for image in images {
            let image = image?;
            let image_name = image.file_name().into_string().unwrap();
            let uuid = image_name.split(".").next().unwrap();
            let mime = mime_guess::from_path(&image.path()).first_or_octet_stream();
            let image = ImagioImage {
                uuid: uuid.to_string(),
                category: category_name.clone(),
                mime,
            };

            let mut stmt = conn.prepare(
                "INSERT INTO images (uuid, category, mime, create_time) VALUES (?, ?, ?, ?)",
            )?;
            let now = chrono::Utc::now();
            stmt.execute(&[
                &image.uuid,
                &image.category,
                &image.mime.to_string(),
                &now.to_string(),
            ])?;
        }
    }
    Ok(())
}