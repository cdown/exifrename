use std::fmt::Write;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use exif::{DateTime, Exif, In, Reader, Tag, Value};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The format to apply to files, excluding the extension.
    ///
    /// Available formats:
    ///
    /// DATETIME:
    ///
    /// %Y  year    (width: 4)
    /// %m  month   (width: 2)
    /// %d  day     (width: 2)
    /// %H  hour    (width: 2)
    /// %M  minute  (width: 2)
    /// %S  second  (width: 2)
    ///
    /// EXPOSURE:
    ///
    /// %f  f-stop
    /// %i  ISO
    /// %s  shutter speed/exposure time, with "/" replaced with "_"
    ///
    /// LITERAL:
    ///
    /// %%  A literal "%"
    #[arg(short, long, verbatim_doc_comment)]
    fmt: String,

    #[arg(
        long,
        help = "Don't actually rename files, only display what would happen"
    )]
    dry_run: bool,

    files: Vec<PathBuf>,
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

fn render_format(exif: &Exif, fmt: &str) -> Result<String> {
    let mut chars = fmt.chars();
    let mut in_fmt = false;
    let dt = get_datetime(exif);
    let nodt = "datetime requested, but not available";

    // Ballpark guess large enough to usually avoid extra allocations
    let mut out = String::with_capacity(fmt.len() * 3);

    while let Some(cur) = chars.next() {
        if !in_fmt {
            if cur == '%' {
                in_fmt = true;
            } else {
                out.push(cur);
            }
            continue;
        }

        in_fmt = false;

        match cur {
            '%' => out.push('%'),

            // DateTime
            'Y' => write!(&mut out, "{:04}", dt.as_ref().context(nodt)?.year)?,
            'm' => write!(&mut out, "{:02}", dt.as_ref().context(nodt)?.month)?,
            'd' => write!(&mut out, "{:02}", dt.as_ref().context(nodt)?.day)?,
            'H' => write!(&mut out, "{:02}", dt.as_ref().context(nodt)?.hour)?,
            'M' => write!(&mut out, "{:02}", dt.as_ref().context(nodt)?.minute)?,
            'S' => write!(&mut out, "{:02}", dt.as_ref().context(nodt)?.second)?,

            // Direct maps to tags
            _ => {
                let tag = match cur {
                    // Exposure attributes
                    'f' => Tag::FNumber,
                    'i' => Tag::PhotographicSensitivity, // TODO: check SensitivityType/0x8830?
                    's' => Tag::ExposureTime, // non-APEX, which has a useful display value

                    _ => panic!("unknown format %{}", cur),
                };

                let field = exif
                    .get_field(tag, In::PRIMARY)
                    .with_context(|| format!("no data for %{}", cur))?;

                write!(&mut out, "{}", field.display_value().to_string().replace("/", "_"))?;
            }
        };
    }

    Ok(out)
}

fn rename_creating_dirs(from: &PathBuf, to_raw: impl Into<PathBuf>) -> Result<()> {
    let to = to_raw.into();
    fs::create_dir_all(&to.parent().context("refusing to move to filesystem root")?)?;

    // Trying to rename cross device? Just copy and unlink the old one
    let ren = fs::rename(&from, &to);
    if let Err(ref err) = ren {
        if let Some(os_err) = err.raw_os_error() {
            if os_err == libc::EXDEV {
                fs::copy(&from, &to)?;
                fs::remove_file(&from)?;
            } else {
                ren?;
            }
        }
    }
    Ok(())
}

fn get_new_name(path: &PathBuf, fmt: &str) -> Result<String> {
    let file = fs::File::open(path)?;
    let exif = Reader::new().read_from_container(&mut BufReader::new(&file))?;

    let mut name = render_format(&exif, fmt)?;
    if let Some(ext) = path.extension() {
        write!(&mut name, ".{}", ext.to_str().context("non-utf8 extension")?)?;
    }
    Ok(name)
}

fn main() -> Result<()> {
    let args = Args::parse();
    for file in args.files {
        match get_new_name(&file, &args.fmt) {
            Ok(new_name) => {
                println!("{} -> {}", file.display(), get_new_name(&file, &args.fmt)?);
                if !args.dry_run {
                    rename_creating_dirs(&file, new_name)?;
                }
            }

            // Fatal conditions like invalid formats go through panic!(), not here
            Err(err) => eprintln!("{}: {}", file.display(), err),
        }
    }
    Ok(())
}
