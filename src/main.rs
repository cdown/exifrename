use std::ffi::CString;
use std::fmt::Write;
use std::fs;
use std::io;
use std::io::BufReader;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, Result};
use clap::Parser;
use tempfile::NamedTempFile;

use exif::{DateTime, Exif, In, Reader, Tag, Value};

type FormatterCallback = fn(&Exif) -> Option<String>;
type DatetimeCallback = fn(&DateTime) -> String;

// This is super small: even with thousands of lookups using a phf::Map is slower
static FORMATTERS: &[(&str, FormatterCallback)] = &[
    ("fstop", |e| get_field(e, Tag::FNumber)),
    ("iso", |e| get_field(e, Tag::PhotographicSensitivity)), // TODO: check SensitivityType/0x8830?
    ("sspeed", |e| get_field(e, Tag::ExposureTime)), // non-APEX, which has a useful display value
    ("year", |e| get_datetime_field(e, |d| format!("{}", d.year))),
    ("year2", |e| get_datetime_field(e, |d| format!("{}", d.year % 100))),
    ("month", |e| get_datetime_field(e, |d| format!("{:02}", d.month))),
    ("day", |e| get_datetime_field(e, |d| format!("{:02}", d.day))),
    ("hour", |e| get_datetime_field(e, |d| format!("{:02}", d.hour))),
    ("minute", |e| get_datetime_field(e, |d| format!("{:02}", d.minute))),
    ("second", |e| get_datetime_field(e, |d| format!("{:02}", d.second))),
];

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The format to apply to files, excluding the extension. Substitutions can be applied inside
    /// curly brackets, for example with {year2} to get the two digit year.
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
    ///   sspeed  (shutter speed/exposure time, with "/" replaced with "_")
    ///
    /// LITERAL:
    ///
    ///   {{ and }} indicate literal brackets.
    #[arg(short, long, verbatim_doc_comment)]
    fmt: String,

    #[arg(
        long,
        help = "Don't actually rename files, only display what would happen"
    )]
    dry_run: bool,

    #[arg(long, help = "Allow overwriting existing files with the same name")]
    overwrite: bool,

    files: Vec<PathBuf>,
}

macro_rules! die {
    ($($arg:tt)*) => {{
        eprintln!("fatal: {}", format!($($arg)*));
        process::exit(1);
    }}
}

fn get_field(exif: &Exif, tag: Tag) -> Option<String> {
    Some(
        exif.get_field(tag, In::PRIMARY)?
            .display_value()
            .to_string()
            .replace('/', "_"),
    )
}

fn get_datetime_field(exif: &Exif, cb: DatetimeCallback) -> Option<String> {
    Some(cb(get_datetime(exif).as_ref()?))
}

// TODO: caching?
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
                            Some(cb) => cb(exif)
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

fn rename(from: &Path, to: &Path, overwrite: bool) -> io::Result<()> {
    let from_c = CString::new(from.as_os_str().as_bytes()).expect("invalid rename source");
    let to_c = CString::new(to.as_os_str().as_bytes()).expect("invalid rename dest");
    let flags = if overwrite { 0 } else { libc::RENAME_NOREPLACE };

    let ret = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            from_c.as_ptr(),
            libc::AT_FDCWD,
            to_c.as_ptr(),
            flags,
        )
    };

    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn rename_creating_dirs(from: &Path, to_raw: impl Into<PathBuf>, overwrite: bool) -> Result<()> {
    let to = to_raw.into();
    let to_parent = to.parent().context("refusing to move to filesystem root")?;
    fs::create_dir_all(to_parent)?;

    // Trying to rename cross device? Just copy and unlink the old one
    let ren = rename(from, &to, overwrite);
    if let Err(ref err) = ren {
        if let Some(os_err) = err.raw_os_error() {
            if os_err == libc::EXDEV {
                let tmp_path = NamedTempFile::new_in(to_parent)?.into_temp_path();
                fs::copy(from, &tmp_path)?;
                rename(&tmp_path, &to, overwrite)?;
                fs::remove_file(from)?;
            } else {
                ren?;
            }
        } else {
            ren?;
        }
    }
    Ok(())
}

fn get_new_name(path: &Path, fmt: &str) -> Result<String> {
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
                    rename_creating_dirs(&file, new_name, args.overwrite)?;
                }
            }

            // Fatal conditions like invalid formats go through panic!(), not here
            Err(err) => eprintln!("{}: {}", file.display(), err),
        }
    }
    Ok(())
}
