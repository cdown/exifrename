use std::collections::HashMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use pbr::ProgressBar;

mod file;
mod format;
mod metadata;
mod types;
mod util;

fn handle_name(
    cfg: &types::Config,
    to_from: &mut HashMap<String, Vec<PathBuf>>,
    file: &Path,
    fp: &Vec<types::FormatPiece>,
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
        write!(&mut to_mod, "_{:0width$}", cnt, width = cnt_width)?;
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
    let cfg = types::Config::parse();
    let mut to_from = HashMap::new();
    let fp = format::format_to_formatpieces(&cfg.fmt)?;

    let mut pb = ProgressBar::new(cfg.files.len() as u64);
    pb.set_max_refresh_rate(Some(Duration::from_millis(100)));
    pb.message("Reading EXIF data: ");

    for file in &cfg.files {
        if let Err(err) = handle_name(&cfg, &mut to_from, file, &fp) {
            eprintln!("failed to get new name for {}: {}", file.display(), err);
            continue;
        }
        pb.inc();
    }

    pb.finish_println("");

    let mut pb = ProgressBar::new(cfg.files.len() as u64);
    pb.set_max_refresh_rate(Some(Duration::from_millis(100)));
    pb.message(&format!(
        "{} files: ",
        if cfg.copy {
            "    Copying"
        } else {
            "   Renaming"
        }
    ));

    for (to_, froms) in to_from {
        // Starts from 0
        let cnt_width = util::get_usize_len(froms.len() - 1);
        for (cnt, from) in froms.iter().enumerate() {
            let to = match finalise_name(&cfg, from, to_.clone(), cnt, cnt_width) {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("failed to finalise {} -> {}: {}", from.display(), to_, err);
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
                continue;
            }
            if cfg.verbose {
                println!("{} -> {}", from.display(), to);
            } else {
                pb.inc();
            }
        }

        pb.finish();
    }

    Ok(())
}
