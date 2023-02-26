use std::str;

use crate::types;
use exif::{DateTime, Exif, In, Tag, Value};

pub fn get_exif_field(im: &types::ImageMetadata, tag: Tag) -> Option<String> {
    let mut out = None;

    if let Some(field) = im.exif.get_field(tag, In::PRIMARY) {
        match field.value {
            // Default formatter puts ASCII values inside quotes, which we don't want
            Value::Ascii(ref vec) if !vec.is_empty() => {
                if let Ok(val) = str::from_utf8(&vec[0]) {
                    out = Some(val.to_string());
                }
            }
            _ => out = Some(field.display_value().to_string()),
        }
    }

    out.map(|s| s.replace('/', "_"))
}

pub fn get_original_filename(im: &types::ImageMetadata) -> Option<String> {
    Some(im.path.file_stem()?.to_str()?.to_string())
}

pub fn get_datetime_field(
    im: &types::ImageMetadata,
    cb: types::DatetimeCallback,
) -> Option<String> {
    im.datetime.as_ref().map(cb)
}

pub fn get_datetime(exif: &Exif) -> Option<DateTime> {
    if let Some(field) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
        match field.value {
            Value::Ascii(ref vec) if !vec.is_empty() => {
                if let Ok(datetime) = DateTime::from_ascii(&vec[0]) {
                    return Some(datetime);
                }
            }
            _ => {}
        }
    }

    None
}
