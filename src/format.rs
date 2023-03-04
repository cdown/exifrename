use std::fmt::Write;
use std::fs;
use std::io;
use std::path::Path;
use std::str;

use anyhow::{bail, Context, Result};

use crate::metadata::{get_datetime, get_datetime_field, get_exif_field, get_original_filename};
use crate::types;
use exif::{Reader, Tag};

macro_rules! fm {
    ($name:tt, $cb:expr) => {
        types::Formatter {
            name: $name,
            cb: $cb,
        }
    };
}

// This is super small: even with thousands of lookups using a phf::Map is slower. Try to order
// more commonly requested fields higher.
static FORMATTERS: &[types::Formatter] = &[
    // Date/time attributes
    fm!("year", |im| get_datetime_field(im, |d| format!("{}", d.year))),
    fm!("year2", |im| get_datetime_field(im, |d| format!("{}", d.year % 100))),
    fm!("month", |im| get_datetime_field(im, |d| format!("{:02}", d.month))),
    fm!("day", |im| get_datetime_field(im, |d| format!("{:02}", d.day))),
    fm!("hour", |im| get_datetime_field(im, |d| format!("{:02}", d.hour))),
    fm!("minute", |im| get_datetime_field(im, |d| format!("{:02}", d.minute))),
    fm!("second", |im| get_datetime_field(im, |d| format!("{:02}", d.second))),
    // Exposure attributes
    fm!("fstop", |im| get_exif_field(im, Tag::FNumber)),
    fm!("iso", |im| get_exif_field(im, Tag::PhotographicSensitivity)), // TODO: check SensitivityType/0x8830?
    fm!("shutter_speed", |im| get_exif_field(im, Tag::ExposureTime)), // non-APEX, which has a useful display value
    // Camera attributes
    fm!("camera_make", |im| get_exif_field(im, Tag::Make)),
    fm!("camera_model", |im| get_exif_field(im, Tag::Model)),
    fm!("camera_serial", |im| get_exif_field(im, Tag::BodySerialNumber)),
    // Lens attributes
    fm!("lens_make", |im| get_exif_field(im, Tag::LensMake)),
    fm!("lens_model", |im| get_exif_field(im, Tag::LensModel)),
    fm!("lens_serial", |im| get_exif_field(im, Tag::LensSerialNumber)),
    fm!("focal_length", |im| get_exif_field(im, Tag::FocalLength)),
    fm!("focal_length_35", |im| get_exif_field(im, Tag::FocalLengthIn35mmFilm)),
    // Filesystem attributes
    fm!("filename", get_original_filename),
];

fn render_format(im: &types::ImageMetadata, fmt: &Vec<types::FormatPiece>) -> Result<String> {
    // Ballpark guess large enough to usually avoid extra allocations
    let mut out = String::with_capacity(fmt.len().checked_mul(4).expect("overflow"));
    for part in fmt {
        match *part {
            types::FormatPiece::Char(c) => out.push(c),
            types::FormatPiece::Fmt(f) => write!(
                &mut out,
                "{}",
                (f.cb)(im).with_context(|| format!("missing data for field '{}'", f.name))?
            )?,
        }
    }
    Ok(out)
}

pub fn format_to_formatpieces(fmt: &str) -> Result<Vec<types::FormatPiece>> {
    let mut chars = fmt.chars().peekable();
    let mut in_fmt = false;

    // Ballpark guesses large enough to usually avoid extra allocations
    let mut out = Vec::with_capacity(fmt.len());
    let mut word = String::with_capacity(16);

    while let Some(cur) = chars.next() {
        if cur == '}' {
            if chars.next_if_eq(&'}').is_some() {
                out.push(types::FormatPiece::Char(cur));
            } else {
                if in_fmt {
                    match FORMATTERS.iter().find(|&f| f.name == word) {
                        Some(f) => out.push(types::FormatPiece::Fmt(f)),
                        None => bail!("invalid field: '{{{word}}}'"),
                    };
                    word.clear();
                    in_fmt = false;
                    continue;
                }
                bail!("mismatched '}}' in format");
            }
        }

        if !in_fmt {
            if cur == '{' {
                in_fmt = true;
            } else {
                out.push(types::FormatPiece::Char(cur));
            }

            continue;
        }

        if cur == '{' {
            if word.is_empty() {
                out.push(types::FormatPiece::Char(cur));
                in_fmt = false;
                continue;
            }
            bail!("nested '{{' in format");
        }

        word.push(cur);
    }

    Ok(out)
}

pub fn get_new_name(path: &Path, fp: &Vec<types::FormatPiece>) -> Result<String> {
    let file = fs::File::open(path)?;
    let exif = Reader::new().read_from_container(&mut io::BufReader::new(&file))?;
    let dt = get_datetime(&exif);
    let im = types::ImageMetadata {
        exif,
        datetime: dt,
        path: path.to_path_buf(),
    };
    render_format(&im, fp)
}
