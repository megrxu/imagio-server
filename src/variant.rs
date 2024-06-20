use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::io::Reader as ImageReader;
use image::{ColorType, GenericImageView, ImageEncoder};
use std::fs::File;
use std::io::{BufWriter, Read, Write};

use fast_image_resize::images::Image;
use fast_image_resize::{IntoImageView, ResizeOptions, Resizer};

use axum::body::{Body, Bytes};
use serde::{Deserialize, Serialize};

use crate::app::ImagioImage;
use crate::ImagioError;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Variant {
    Public,
    Embed,
    Thumb,
    Banner,
    Square,
    #[default]
    Original,
}

impl From<&str> for Variant {
    fn from(s: &str) -> Self {
        match s {
            "public" => Variant::Public,
            "thumb" => Variant::Thumb,
            "banner" => Variant::Banner,
            "square" => Variant::Square,
            "embed" => Variant::Embed,
            _ => Variant::Original,
        }
    }
}

impl ToString for Variant {
    fn to_string(&self) -> String {
        match self {
            Variant::Public => "public".to_string(),
            Variant::Thumb => "thumb".to_string(),
            Variant::Square => "square".to_string(),
            Variant::Banner => "banner".to_string(),
            Variant::Embed => "embed".to_string(),
            Variant::Original => "original".to_string(),
        }
    }
}

pub trait Original {
    fn original(&self) -> String;
}

pub trait ImageVariant {
    fn variant(&self, variant: Variant) -> Result<Body, ImagioError>;
}

impl Original for ImagioImage {
    fn original(&self) -> String {
        let ext = self.mime.subtype().to_string().to_uppercase();
        format!("data/images/{}/{}.{}", self.category, self.uuid, ext)
    }
}

impl Variant {
    pub fn transform(&self, path: &str) -> Bytes {
        let img = ImageReader::open(path).unwrap().decode().unwrap();
        let (width, height) = img.dimensions();
        // Create container for data of destination image
        let (dst_width, dst_height) = match self {
            Variant::Public => (1024, 768),
            Variant::Embed => (width.min(1024), height * width.min(1024) / width),
            Variant::Thumb => (256, 256),
            Variant::Banner => (800, 400),
            Variant::Square => (320, 320),
            Variant::Original => unreachable!(),
        };
        // Create container for data of destination image
        let mut dst_image = Image::new(dst_width, dst_height, img.pixel_type().unwrap());

        tracing::info!("Starting transformation of {} to {:?}", path, self);
        let mut resizer = Resizer::new();
        resizer
            .resize(
                &img,
                &mut dst_image,
                &ResizeOptions::new().fit_into_destination(None),
            )
            .unwrap();
        tracing::info!("Finished transformation of {} to {:?}", path, self);

        // Write destination image as PNG-file
        tracing::info!("Starting encoding to Jpeg.");
        let mut result_buf = BufWriter::new(Vec::new());
        match img.color() {
            ColorType::Rgba8 | ColorType::Rgb16 => {
                PngEncoder::new(&mut result_buf)
                    .write_image(
                        dst_image.buffer(),
                        dst_width,
                        dst_height,
                        img.color().into(),
                    )
                    .unwrap();
            }
            _ => {
                JpegEncoder::new(&mut result_buf)
                    .write_image(
                        dst_image.buffer(),
                        dst_width,
                        dst_height,
                        img.color().into(),
                    )
                    .unwrap();
            }
        }
        tracing::info!("Finished encoding to Jpeg.");

        // Return the bytes in the buffer
        Bytes::from(result_buf.into_inner().unwrap())
    }
}

impl ImageVariant for ImagioImage {
    fn variant(&self, variant: Variant) -> Result<Body, ImagioError> {
        let original = self.original();
        match variant {
            Variant::Original => {
                let mut file = File::open(original).unwrap();
                let mut contents = Vec::new();
                file.read_to_end(&mut contents).unwrap();
                Ok(Body::from(Bytes::from(contents)))
            }
            variant => {
                // check if the cached file exists
                if let Ok(mut file) = File::open(format!(
                    "data/cache/{}_{}_{}.{}",
                    self.category,
                    self.uuid,
                    variant.to_string(),
                    self.mime.subtype()
                )) {
                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents)?;
                    return Ok(Body::from(Bytes::from(contents)));
                }
                tracing::info!(
                    "Transforming {} to variant {}.",
                    self.uuid,
                    variant.to_string()
                );
                let bytes = variant.transform(&original);
                // write the bytes to the cached file
                let mut file = File::create(format!(
                    "data/cache/{}_{}_{}.jpeg",
                    self.category,
                    self.uuid,
                    variant.to_string(),
                ))?;
                file.write_all(&bytes)?;
                Ok(Body::from(bytes))
            }
        }
    }
}

pub fn generate() -> Result<(), ImagioError> {
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

            for variant in [Variant::Public, Variant::Square, Variant::Embed] {
                let _ = image.variant(variant);
            }
        }
    }
    Ok(())
}
