use clap::Parser;
use exif::{DateTime, Exif};
use std::path::PathBuf;

pub type FormatterCallback = fn(&ImageMetadata) -> Option<String>;
pub type DatetimeCallback = fn(&DateTime) -> String;

pub struct ImageMetadata {
    pub exif: Exif,
    pub datetime: Option<DateTime>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Config {
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
    pub fmt: String,

    #[arg(
        short,
        long = "counter",
        help = "Append a counter of this width to each format"
    )]
    pub counter_width: Option<usize>,

    #[arg(
        long,
        help = "Don't actually rename files, only display what would happen"
    )]
    pub dry_run: bool,

    #[arg(long, help = "Allow overwriting existing files with the same name")]
    pub overwrite: bool,

    #[arg(short = 'o', long, help = "Copy instead of renaming")]
    pub copy: bool,

    pub files: Vec<PathBuf>,
}
