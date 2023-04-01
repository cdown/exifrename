use std::fs;
use std::io;
use std::path::Path;

use anyhow::Result;

use crate::metadata::get_datetime;
use crate::types;
use exif::Reader;

pub fn get_new_name(path: &Path, fp: &Vec<ftempl::FormatPiece<types::ImageMetadata>>) -> Result<String> {
    let file = fs::File::open(path)?;
    let exif = Reader::new().read_from_container(&mut io::BufReader::new(&file))?;
    let dt = get_datetime(&exif);
    let im = types::ImageMetadata {
        exif,
        datetime: dt,
        path: path.to_path_buf(),
    };

    let ret = Ok(ftempl::render(&im, fp)?);

    ret
}
