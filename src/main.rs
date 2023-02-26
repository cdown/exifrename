use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::str;

use anyhow::{Context, Result};
use clap::Parser;

use exif::{DateTime, Exif, In, Reader, Tag, Value};

mod file;

type FormatterCallback = fn(&ImageMetadata) -> Option<String>;
type DatetimeCallback = fn(&DateTime) -> String;

struct ImageMetadata {
    exif: Exif,
    datetime: Option<DateTime>,
}

// This is super small: even with thousands of lookups using a phf::Map is slower. Try to order
// more commonly requested fields higher.
static FORMATTERS: &[(&str, FormatterCallback)] = &[
    // Date/time attributes
    ("year", |im| get_datetime_field(im, |d| format!("{}", d.year))),
    ("year2", |im| get_datetime_field(im, |d| format!("{}", d.year % 100))),
    ("month", |im| get_datetime_field(im, |d| format!("{:02}", d.month))),
    ("day", |im| get_datetime_field(im, |d| format!("{:02}", d.day))),
    ("hour", |im| get_datetime_field(im, |d| format!("{:02}", d.hour))),
    ("minute", |im| get_datetime_field(im, |d| format!("{:02}", d.minute))),
    ("second", |im| get_datetime_field(im, |d| format!("{:02}", d.second))),
    // Exposure attributes
    ("fstop", |im| get_field(im, Tag::FNumber)),
    ("iso", |im| get_field(im, Tag::PhotographicSensitivity)), // TODO: check SensitivityType/0x8830?
    ("shutter_speed", |im| get_field(im, Tag::ExposureTime)), // non-APEX, which has a useful display value
    // Camera attributes
    ("camera_make", |im| get_field(im, Tag::Make)),
    ("camera_model", |im| get_field(im, Tag::Model)),
    ("camera_serial", |im| get_field(im, Tag::BodySerialNumber)),
    // Lens attributes
    ("lens_make", |im| get_field(im, Tag::LensMake)),
    ("lens_model", |im| get_field(im, Tag::LensModel)),
    ("lens_serial", |im| get_field(im, Tag::LensSerialNumber)),
    ("focal_length", |im| get_field(im, Tag::FocalLength)),
    ("focal_length_35", |im| get_field(im, Tag::FocalLengthIn35mmFilm)),
];

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

macro_rules! die {
    ($($arg:tt)*) => {{
        eprintln!("fatal: {}", format!($($arg)*));
        process::exit(1);
    }}
}

fn get_field(im: &ImageMetadata, tag: Tag) -> Option<String> {
    let mut out = None;

    if let Some(field) = im.exif.get_field(tag, In::PRIMARY) {
        match field.value {
            // Default formatter puts ASCII values inside quotes, which we don't want
            Value::Ascii(ref vec) if !vec.is_empty() => {
                if let Ok(val) = str::from_utf8(&vec[0]) {
                    out = Some(val.to_string());
                }
            }
            _ => out = Some(field.display_value().to_string()),
        }
    }

    out.map(|s| s.replace('/', "_"))
}

fn get_datetime_field(im: &ImageMetadata, cb: DatetimeCallback) -> Option<String> {
    im.datetime.as_ref().map(cb)
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

fn render_format(im: &ImageMetadata, fmt: &str) -> Result<String> {
    let mut chars = fmt.chars().peekable();
    let mut in_fmt = false;

    // Ballpark guesses large enough to usually avoid extra allocations
    let mut out = String::with_capacity(fmt.len() * 3);
    let mut word = String::with_capacity(16);

    while let Some(cur) = chars.next() {
        if cur == '}' {
            match chars.next_if_eq(&'}') {
                Some(_) => out.push(cur),
                None => {
                    if in_fmt {
                        let rep = match FORMATTERS
                            .iter()
                            .find(|&&(s, _)| s == word)
                            .map(|&(_, f)| f)
                        {
                            Some(cb) => cb(im)
                                .with_context(|| format!("missing data for field '{}'", word))?,
                            None => die!("invalid field: '{{{}}}'", word),
                        };
                        word.clear();
                        write!(&mut out, "{}", rep)?;
                        in_fmt = false;
                        continue;
                    } else {
                        die!("mismatched '}}' in format");
                    }
                }
            }
        }

        if !in_fmt {
            if cur == '{' {
                in_fmt = true;
            } else {
                out.push(cur);
            }

            continue;
        }

        if cur == '{' {
            if word.is_empty() {
                out.push(cur);
                in_fmt = false;
                continue;
            } else {
                die!("nested '{{' in format");
            }
        }

        word.push(cur);
    }

    Ok(out)
}

fn get_new_name(
    path: &Path,
    fmt: &str,
    counter: &mut HashMap<String, u16>,
    width: Option<usize>,
) -> Result<String> {
    let file = fs::File::open(path)?;
    let exif = Reader::new().read_from_container(&mut io::BufReader::new(&file))?;
    let dt = get_datetime(&exif);
    let im = ImageMetadata { exif, datetime: dt };
    let mut name = render_format(&im, fmt)?;

    if let Some(pad) = width {
        let cnt = counter.entry(name.clone()).or_default();
        *cnt += 1;
        write!(&mut name, "_{:0width$}", *cnt, width = pad)?;
    }

    if let Some(ext) = path.extension() {
        write!(&mut name, ".{}", ext.to_str().context("non-utf8 extension")?)?;
    }

    Ok(name)
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
