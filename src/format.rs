use std::fs;
use std::io;
use std::path::Path;

use anyhow::Result;

use crate::metadata::get_datetime;
use crate::types;
use exif::Reader;
use funcfmt::{FormatPieces, Render};

pub fn get_new_name(path: &Path, fp: &FormatPieces<types::ImageMetadata>) -> Result<String> {
    let file = fs::File::open(path)?;
    let exif = Reader::new()
        .read_from_container(&mut io::BufReader::new(&file))
        .map_err(|e| anyhow::anyhow!("Failed to read EXIF data: {:?}", e))?;
    let dt = get_datetime(&exif);
    let im = types::ImageMetadata {
        exif,
        datetime: dt,
        path: path.to_path_buf(),
    };
    Ok(fp.render(&im)?)
}
