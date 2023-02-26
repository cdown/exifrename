use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

mod file;
mod format;
mod metadata;
mod types;
mod util;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The format to apply to files, excluding the extension. Substitutions can be applied inside
    /// curly brackets, for example with {year2} to get the two digit year. Any formats returning
    /// data with "/" will have it transformed to "_".
    ///
    /// Available formats:
    ///
    /// DATETIME:
    ///
    ///   year    (width: 4)
    ///   year2   (width: 2)
    ///   month   (width: 2)
    ///   day     (width: 2)
    ///   hour    (width: 2)
    ///   minute  (width: 2)
    ///   second  (width: 2)
    ///
    /// EXPOSURE:
    ///
    ///   fstop
    ///   iso
    ///   shutter_speed
    ///
    ///
    /// CAMERA:
    ///
    ///   camera_make
    ///   camera_model
    ///   camera_serial
    ///
    /// LENS:
    ///
    ///   lens_make
    ///   lens_model
    ///   lens_serial
    ///   focal_length
    ///   focal_length_35  (Focal length in 35mm equivalent)
    ///
    /// LITERAL:
    ///
    ///   {{ and }} indicate literal brackets.
    #[arg(short, long, verbatim_doc_comment)]
    fmt: String,

    #[arg(short, long, help = "Append a counter of this width to each format")]
    counter: Option<usize>,

    #[arg(
        long,
        help = "Don't actually rename files, only display what would happen"
    )]
    dry_run: bool,

    #[arg(long, help = "Allow overwriting existing files with the same name")]
    overwrite: bool,

    #[arg(short = 'o', long, help = "Copy instead of renaming")]
    copy: bool,

    files: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut counter: HashMap<String, u16> = HashMap::new();

    for file in args.files {
        match get_new_name(&file, &args.fmt, &mut counter, args.counter) {
            Ok(new_name) => {
                println!("{} -> {}", file.display(), new_name);
                if !args.dry_run {
                    if args.copy {
                        file::copy_creating_dirs(&file, new_name, args.overwrite)?;
                    } else {
                        file::rename_creating_dirs(&file, new_name, args.overwrite)?;
                    }
                }
            }

            // Fatal conditions like invalid formats go through panic!(), not here
            Err(err) => eprintln!("{}: {}", file.display(), err),
        }
    }
    Ok(())
}
