use std::collections::HashMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

mod file;
mod format;
mod metadata;
mod types;
mod util;

fn handle_name(
    to_from: &mut HashMap<String, Vec<PathBuf>>,
    file: &Path,
    fp: &Vec<types::FormatPiece>,
) -> Result<()> {
    let new_name = format::get_new_name(file, fp)?;
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
    println!("{} -> {}", from.display(), to);
    if !cfg.dry_run {
        if cfg.copy {
            file::copy_creating_dirs(from, to, cfg.overwrite)?;
        } else {
            file::rename_creating_dirs(from, to, cfg.overwrite)?;
        }
    }
    Ok(())
}

fn handle_finalise(
    cfg: &types::Config,
    from: &Path,
    to: String,
    cnt: usize,
    cnt_width: usize,
) -> Result<()> {
    let to = finalise_name(cfg, from, to, cnt, cnt_width)?;
    handle_file(cfg, from, &to)
}

fn main() -> Result<()> {
    let cfg = types::Config::parse();
    let mut to_from = HashMap::new();
    let fp = format::format_to_formatpieces(&cfg.fmt)?;

    for file in &cfg.files {
        if let Err(err) = handle_name(&mut to_from, file, &fp) {
            eprintln!("failed to get new name for {}: {}", file.display(), err);
        }
    }

    for (to, froms) in to_from {
        // Starts from 0
        let cnt_width = util::get_usize_len(froms.len() - 1);
        for (cnt, from) in froms.iter().enumerate() {
            if let Err(err) = handle_finalise(&cfg, from, to.clone(), cnt, cnt_width) {
                eprintln!("failed to finalise {} -> {}: {}", from.display(), to, err);
            }
        }
    }

    Ok(())
}
