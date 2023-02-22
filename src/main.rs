use std::fmt::Write;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Parser;

use exif::{DateTime, Exif, In, Reader, Tag, Value};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    fmt: String,

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

fn nodt(path: &PathBuf) -> String {
    format!("{}: datetime requested, but not available", path.display())
}

fn render_format(path: &PathBuf, exif: &Exif, fmt: &str) -> Result<String> {
    let mut chars = fmt.chars();
    let mut in_fmt = false;
    let dt = get_datetime(exif);

    // Currently cannot go over this, since widest DT field (year) is 2x input
    let mut out = String::with_capacity(fmt.len() * 2);

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
            'Y' => write!(&mut out, "{:04}", dt.as_ref().with_context(|| nodt(path))?.year)?,
            'm' => write!(&mut out, "{:02}", dt.as_ref().with_context(|| nodt(path))?.month)?,
            'd' => write!(&mut out, "{:02}", dt.as_ref().with_context(|| nodt(path))?.day)?,
            'H' => write!(&mut out, "{:02}", dt.as_ref().with_context(|| nodt(path))?.hour)?,
            'M' => write!(&mut out, "{:02}", dt.as_ref().with_context(|| nodt(path))?.minute)?,
            'S' => write!(&mut out, "{:02}", dt.as_ref().with_context(|| nodt(path))?.second)?,

            // Direct maps to tags
            _ => {
                let tag = match cur {
                    // Exposure attributes
                    'f' => Tag::FNumber,
                    'i' => Tag::PhotographicSensitivity, // TODO: check SensitivityType/0x8830?
                    's' => Tag::ExposureTime, // non-APEX, which has a useful display value

                    _ => bail!("unknown format %{}", cur),
                };

                let field = exif
                    .get_field(tag, In::PRIMARY)
                    .with_context(|| format!("no data for %{}", cur))?;

                write!(&mut out, "{}", field.display_value())?;
            }
        };
    }

    Ok(out)
}

fn get_new_name(path: &PathBuf, fmt: &str) -> Result<String> {
    let file = File::open(path)?;
    let exif = Reader::new().read_from_container(&mut BufReader::new(&file))?;

    render_format(&path, &exif, fmt)
}

fn main() -> Result<()> {
    let args = Args::parse();
    for file in args.files {
        println!("{}", get_new_name(&file, &args.fmt)?);
    }

    Ok(())
}
