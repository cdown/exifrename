use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use exif::{DateTime, Exif, In, Reader, Tag, Value};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    fmt: String,

    #[arg(value_parser)]
    files: Vec<PathBuf>,
}

fn get_datetime(exif: &Exif) -> Option<DateTime> {
    if let Some(field) = exif.get_field(Tag::DateTime, In::PRIMARY) {
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

fn missing_dt(path: &PathBuf) -> u16 {
    eprintln!("{}: missing datetime information", path.display());
    0
}

fn render_format(path: &PathBuf, exif: &Exif, fmt: &str) -> String {
    let mut out = vec![];
    let mut chars = fmt.chars();
    let mut in_fmt = false;

    while let Some(cur) = chars.next() {
        if !in_fmt {
            if cur == '%' {
                in_fmt = true;
            } else {
                out.push(cur.to_string());
            }
            continue;
        }

        in_fmt = false;

        match cur {
            '%' => out.push("%".to_string()),
            'Y' => out.push(
                get_datetime(exif)
                    .map_or_else(|| missing_dt(path), |d| d.year)
                    .to_string(),
            ),
            _ => {
                eprintln!("ignored unknown format %{}", cur);
                out.push(format!("%{}", cur))
            }
        };
    }

    out.join("")
}

fn get_new_name(path: &PathBuf, fmt: &str) -> Result<String> {
    let file = File::open(path)?;
    let exif = Reader::new().read_from_container(&mut BufReader::new(&file))?;

    Ok(render_format(&path, &exif, fmt))
}

fn main() -> Result<()> {
    let args = Args::parse();
    for file in args.files {
        println!("{}", get_new_name(&file, &args.fmt)?);
    }

    Ok(())
}
