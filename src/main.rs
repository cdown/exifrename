use std::collections::HashMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Parser;
use exif::Tag;
use funcfmt::{fm, FormatMap, FormatPieces, ToFormatPieces};

mod file;
mod format;
mod metadata;
mod types;
mod util;

fn handle_name(
    cfg: &types::Config,
    to_from: &mut HashMap<String, Vec<PathBuf>>,
    file: &Path,
    fp: &FormatPieces<types::ImageMetadata>,
) -> Result<()> {
    let new_name = format::get_new_name(file, fp)?;
    if cfg.verbose {
        println!("Read EXIF from {}, new intermediate format is {}", file.display(), new_name);
    }
    let entry = to_from.entry(new_name).or_insert_with(Vec::new);
    (*entry).push(file.to_path_buf());
    Ok(())
}

fn finalise_name(
    cfg: &types::Config,
    from: &Path,
    to: String,
    cnt: usize,
    cnt_width: usize,
) -> Result<String> {
    let mut to_mod = to;

    if !cfg.no_counter && cnt_width > 0 {
        write!(&mut to_mod, "_{cnt:0cnt_width$}")?;
    }

    if let Some(ext) = from.extension() {
        write!(&mut to_mod, ".{}", ext.to_str().context("non-utf8 extension")?)?;
    }

    Ok(to_mod)
}

fn handle_file(cfg: &types::Config, from: &Path, to: &str) -> Result<()> {
    if !cfg.dry_run {
        if cfg.copy {
            file::copy_creating_dirs(from, to, cfg.overwrite)?;
        } else {
            file::rename_creating_dirs(from, to, cfg.overwrite)?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let formatters: funcfmt::FormatMap<types::ImageMetadata> = FormatMap::from([
        // Date/time attributes
        fm!("year", |im| metadata::get_datetime_field(im, |d| format!("{}", d.year))),
        fm!("year2", |im| metadata::get_datetime_field(im, |d| format!("{}", d.year % 100))),
        fm!("month", |im| metadata::get_datetime_field(im, |d| format!("{:02}", d.month))),
        fm!("day", |im| metadata::get_datetime_field(im, |d| format!("{:02}", d.day))),
        fm!("hour", |im| metadata::get_datetime_field(im, |d| format!("{:02}", d.hour))),
        fm!("minute", |im| metadata::get_datetime_field(im, |d| format!("{:02}", d.minute))),
        fm!("second", |im| metadata::get_datetime_field(im, |d| format!("{:02}", d.second))),
        // Exposure attributes
        fm!("fstop", |im| metadata::get_exif_field(im, Tag::FNumber)),
        fm!("iso", |im| metadata::get_exif_field(im, Tag::PhotographicSensitivity)), // TODO: check SensitivityType/0x8830?
        fm!("shutter_speed", |im| metadata::get_exif_field(im, Tag::ExposureTime)), // non-APEX, which has a useful display value
        // Camera attributes
        fm!("camera_make", |im| metadata::get_exif_field(im, Tag::Make)),
        fm!("camera_model", |im| metadata::get_exif_field(im, Tag::Model)),
        fm!("camera_serial", |im| metadata::get_exif_field(im, Tag::BodySerialNumber)),
        // Lens attributes
        fm!("lens_make", |im| metadata::get_exif_field(im, Tag::LensMake)),
        fm!("lens_model", |im| metadata::get_exif_field(im, Tag::LensModel)),
        fm!("lens_serial", |im| metadata::get_exif_field(im, Tag::LensSerialNumber)),
        fm!("focal_length", |im| metadata::get_exif_field(im, Tag::FocalLength)),
        fm!("focal_length_35", |im| metadata::get_exif_field(im, Tag::FocalLengthIn35mmFilm)),
        // Filesystem attributes
        fm!("filename", metadata::get_original_filename),
    ]);

    let cfg = types::Config::parse();
    let mut to_from = HashMap::new();
    let fp = formatters.to_format_pieces(&cfg.fmt)?;

    let mut error_seen = false;
    for file in &cfg.files {
        if let Err(err) = handle_name(&cfg, &mut to_from, file, &fp) {
            eprintln!("failed to get new name for {}: {}", file.display(), err);
            error_seen = true;
        }
    }

    for (to_, froms) in to_from {
        // Starts from 0
        let cnt_width = util::get_usize_len(froms.len().checked_sub(1).expect("underflow"));
        for (cnt, from) in froms.iter().enumerate() {
            let to = match finalise_name(&cfg, from, to_.clone(), cnt, cnt_width) {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("failed to finalise {} -> {}: {}", from.display(), to_, err);
                    error_seen = true;
                    continue;
                }
            };
            if let Err(err) = handle_file(&cfg, from, &to) {
                eprintln!(
                    "failed to {} {} -> {}: {}",
                    if cfg.copy { "copy" } else { "rename" },
                    from.display(),
                    to,
                    err
                );
                error_seen = true;
                continue;
            }
            println!("{} -> {}", from.display(), to);
        }
    }

    if error_seen {
        bail!("see previously mentioned errors")
    }

    Ok(())
}
