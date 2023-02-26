use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::str;

use anyhow::{Context, Result};
use clap::Parser;

use exif::{DateTime, Exif, In, Reader, Tag, Value};

fn get_field(im: &ImageMetadata, tag: Tag) -> Option<String> {
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

fn get_datetime_field(im: &ImageMetadata, cb: DatetimeCallback) -> Option<String> {
    im.datetime.as_ref().map(cb)
}

fn get_datetime(exif: &Exif) -> Option<DateTime> {
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
