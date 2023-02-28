use std::path::Path;

use anyhow::Result;
use clap::Parser;
use hashbrown::HashMap;

mod file;
mod format;
mod metadata;
mod types;
mod util;

fn handle_file(
    cfg: &types::Config,
    counter: &mut HashMap<String, u16>,
    file: &Path,
    fp: &Vec<types::FormatPiece>,
) -> Result<()> {
    let new_name = format::get_new_name(cfg, file, counter, fp)?;
    println!("{} -> {}", file.display(), new_name);
    if !cfg.dry_run {
        if cfg.copy {
            file::copy_creating_dirs(file, new_name, cfg.overwrite)?;
        } else {
            file::rename_creating_dirs(file, new_name, cfg.overwrite)?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cfg = types::Config::parse();
    let mut counter: HashMap<String, u16> = HashMap::new();
    let fp = format::format_to_formatpiece(&cfg.fmt)?;

    for file in &cfg.files {
        if let Err(err) = handle_file(&cfg, &mut counter, file, &fp) {
            eprintln!("{}: {}", file.display(), err);
        }
    }

    Ok(())
}
