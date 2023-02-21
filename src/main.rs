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

fn get_datetime(path: &PathBuf, exif: &Exif) -> DateTime {
    if let Some(field) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
        match field.value {
            Value::Ascii(ref vec) if !vec.is_empty() => {
                if let Ok(datetime) = DateTime::from_ascii(&vec[0]) {
                    return datetime;
                }
            }
            _ => {}
        }
    }

    eprintln!("{}: missing datetime information", path.display());

    DateTime {
        year: 0,
        month: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        nanosecond: None,
        offset: None,
    }
}

fn render_format(path: &PathBuf, exif: &Exif, fmt: &str) -> String {
    let mut out = vec![];
    let mut chars = fmt.chars();
    let mut in_fmt = false;
    let dt = get_datetime(path, exif);

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
            'Y' => out.push(dt.year.to_string()),
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
