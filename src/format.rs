use std::fmt::Write;
use std::fs;
use std::io;
use std::path::Path;
use std::str;

use anyhow::{Context, Result};
use hashbrown::HashMap;

use crate::metadata::{get_datetime, get_datetime_field, get_exif_field, get_original_filename};
use crate::{types, util};
use exif::{Reader, Tag};

// This is super small: even with thousands of lookups using a phf::Map is slower. Try to order
// more commonly requested fields higher.
static FORMATTERS: &[(&str, types::FormatterCallback)] = &[
    // Date/time attributes
    ("year", |im| get_datetime_field(im, |d| format!("{}", d.year))),
    ("year2", |im| get_datetime_field(im, |d| format!("{}", d.year % 100))),
    ("month", |im| get_datetime_field(im, |d| format!("{:02}", d.month))),
    ("day", |im| get_datetime_field(im, |d| format!("{:02}", d.day))),
    ("hour", |im| get_datetime_field(im, |d| format!("{:02}", d.hour))),
    ("minute", |im| get_datetime_field(im, |d| format!("{:02}", d.minute))),
    ("second", |im| get_datetime_field(im, |d| format!("{:02}", d.second))),
    // Exposure attributes
    ("fstop", |im| get_exif_field(im, Tag::FNumber)),
    ("iso", |im| get_exif_field(im, Tag::PhotographicSensitivity)), // TODO: check SensitivityType/0x8830?
    ("shutter_speed", |im| get_exif_field(im, Tag::ExposureTime)), // non-APEX, which has a useful display value
    // Camera attributes
    ("camera_make", |im| get_exif_field(im, Tag::Make)),
    ("camera_model", |im| get_exif_field(im, Tag::Model)),
    ("camera_serial", |im| get_exif_field(im, Tag::BodySerialNumber)),
    // Lens attributes
    ("lens_make", |im| get_exif_field(im, Tag::LensMake)),
    ("lens_model", |im| get_exif_field(im, Tag::LensModel)),
    ("lens_serial", |im| get_exif_field(im, Tag::LensSerialNumber)),
    ("focal_length", |im| get_exif_field(im, Tag::FocalLength)),
    ("focal_length_35", |im| get_exif_field(im, Tag::FocalLengthIn35mmFilm)),
    // Filesystem attributes
    ("filename", get_original_filename),
];

pub fn render_format(im: &types::ImageMetadata, fmt: &str) -> Result<String> {
    let mut chars = fmt.chars().peekable();
    let mut in_fmt = false;

    // Ballpark guesses large enough to usually avoid extra allocations
    let mut out = String::with_capacity(fmt.len() * 3);
    let mut word = String::with_capacity(16);

    while let Some(cur) = chars.next() {
        if cur == '}' {
            match chars.next_if_eq(&'}') {
                Some(_) => out.push(cur),
                None => {
                    if in_fmt {
                        let rep = match FORMATTERS
                            .iter()
                            .find(|&&(s, _)| s == word)
                            .map(|&(_, f)| f)
                        {
                            Some(cb) => cb(im)
                                .with_context(|| format!("missing data for field '{word}'"))?,
                            None => util::die!("invalid field: '{{{word}}}'"),
                        };
                        word.clear();
                        write!(&mut out, "{rep}")?;
                        in_fmt = false;
                        continue;
                    } else {
                        util::die!("mismatched '}}' in format");
                    }
                }
            }
        }

        if !in_fmt {
            if cur == '{' {
                in_fmt = true;
            } else {
                out.push(cur);
            }

            continue;
        }

        if cur == '{' {
            if word.is_empty() {
                out.push(cur);
                in_fmt = false;
                continue;
            } else {
                util::die!("nested '{{' in format");
            }
        }

        word.push(cur);
    }

    Ok(out)
}

pub fn get_new_name(
    cfg: &types::Config,
    path: &Path,
    counter: &mut HashMap<String, u16>,
) -> Result<String> {
    let file = fs::File::open(path)?;
    let exif = Reader::new().read_from_container(&mut io::BufReader::new(&file))?;
    let dt = get_datetime(&exif);
    let im = types::ImageMetadata {
        exif,
        datetime: dt,
        path: path.to_path_buf(),
    };
    let mut name = render_format(&im, &cfg.fmt)?;

    if let Some(pad) = cfg.counter_width {
        let cnt = counter
            .raw_entry_mut()
            .from_key(&name)
            .and_modify(|_, c| *c += 1)
            .or_insert_with(|| (name.clone(), 1));
        write!(&mut name, "_{:0width$}", *cnt.1, width = pad)?;
    }

    if let Some(ext) = path.extension() {
        write!(&mut name, ".{}", ext.to_str().context("non-utf8 extension")?)?;
    }

    Ok(name)
}
