use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::io::Reader as ImageReader;
use image::{ColorType, DynamicImage, GenericImageView, ImageEncoder};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;

use fast_image_resize::images::Image;
use fast_image_resize::{IntoImageView, ResizeOptions, Resizer};

use axum::body::{Body, Bytes};
use serde::{Deserialize, Serialize};

use crate::app::ImagioImage;
use crate::{ImagioError, ImagioState};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
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
pub trait ImageVariant {
    fn variant_raw(&self, image: &ImagioImage, variant: Variant) -> Result<Vec<u8>, ImagioError>;
    fn variant(&self, image: &ImagioImage, variant: Variant) -> Result<Bytes, ImagioError> {
        let raw = self.variant_raw(image, variant)?;
        Ok(Bytes::from(raw))
    }
}

impl Variant {
    pub fn transform(&self, img: DynamicImage) -> Bytes {
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

        let mut resizer = Resizer::new();
        resizer
            .resize(
                &img,
                &mut dst_image,
                &ResizeOptions::new().fit_into_destination(None),
            )
            .unwrap();

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

impl ImageVariant for ImagioState {
    fn variant_raw(&self, image: &ImagioImage, variant: Variant) -> Result<Vec<u8>, ImagioError> {
        let original_path = format!("{}/{}", self.store, image.filename(&Variant::Original));
        match variant {
            Variant::Original => {
                let mut file = File::open(original_path).unwrap();
                let mut contents = Vec::new();
                file.read_to_end(&mut contents).unwrap();
                Ok(contents)
            }
            variant => {
                // check if the cached file exists
                let variant_path = format!("{}/{}", self.cache, image.filename(&variant));
                tracing::info!("Checking for cached variant at: {}", variant_path);
                if let Ok(mut file) = File::open(variant_path) {
                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents)?;
                    return Ok(contents);
                }
                let img = ImageReader::open(original_path)?.decode()?;
                let bytes = variant.transform(img);
                // Write the variant image to the store
                image.store(&bytes, self.cache.clone(), image.filename(&variant))?;
                Ok(bytes.to_vec())
            }
        }
    }
}

pub fn generate() -> Result<(), ImagioError> {
    Ok(())
}
