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
    /// %y  year    (width: 2)
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

macro_rules! die {
    ($($arg:tt)*) => {{
        eprintln!("fatal: {}", format!($($arg)*));
        process::exit(1);
    }}
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
            'y' => write!(&mut out, "{:02}", dt.as_ref().context(nodt)?.year % 100)?,
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

                    _ => die!("unknown format %{}", cur),
                };

                let field = exif
                    .get_field(tag, In::PRIMARY)
                    .with_context(|| format!("no data for %{}", cur))?;

                write!(&mut out, "{}", field.display_value().to_string().replace("/", "_"))?;
            }
        };
    }

    if in_fmt {
        die!("unfinished percent string at end of format");
    }

    Ok(out)
}

fn rename_noclobber(from: &Path, to: &Path) -> io::Result<()> {
    let from_c = CString::new(from.as_os_str().as_bytes()).expect("invalid rename source");
    let to_c = CString::new(to.as_os_str().as_bytes()).expect("invalid rename dest");

    let ret = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            from_c.as_ptr(),
            libc::AT_FDCWD,
            to_c.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };

    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn rename_creating_dirs(from: &Path, to_raw: impl Into<PathBuf>) -> Result<()> {
    let to = to_raw.into();
    let to_parent = to.parent().context("refusing to move to filesystem root")?;
    fs::create_dir_all(to_parent)?;

    // Trying to rename cross device? Just copy and unlink the old one
    let ren = rename_noclobber(&from, &to);
    if let Err(ref err) = ren {
        if let Some(os_err) = err.raw_os_error() {
            if os_err == libc::EXDEV {
                let tmp_path = NamedTempFile::new_in(to_parent)?.into_temp_path();
                fs::copy(&from, &tmp_path)?;
                rename_noclobber(&tmp_path, &to)?;
                fs::remove_file(&from)?;
            } else {
                ren?;
            }
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
                    rename_creating_dirs(&file, new_name)?;
                }
            }

            // Fatal conditions like invalid formats go through panic!(), not here
            Err(err) => eprintln!("{}: {}", file.display(), err),
        }
    }
    Ok(())
}
