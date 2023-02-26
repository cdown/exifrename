use clap::Parser;
use std::collections::HashMap;

use anyhow::Result;

mod file;
mod format;
mod metadata;
mod types;
mod util;

fn main() -> Result<()> {
    let cfg = types::Config::parse();
    let mut counter: HashMap<String, u16> = HashMap::new();

    for file in &cfg.files {
        match format::get_new_name(&cfg, file, &mut counter) {
            Ok(new_name) => {
                println!("{} -> {}", file.display(), new_name);
                if !cfg.dry_run {
                    if cfg.copy {
                        file::copy_creating_dirs(file, new_name, cfg.overwrite)?;
                    } else {
                        file::rename_creating_dirs(file, new_name, cfg.overwrite)?;
                    }
                }
            }

            // Fatal conditions like invalid formats go through panic!(), not here
            Err(err) => eprintln!("{}: {}", file.display(), err),
        }
    }
    Ok(())
}
