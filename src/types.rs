use exif::{DateTime, Exif};

pub type FormatterCallback = fn(&ImageMetadata) -> Option<String>;
pub type DatetimeCallback = fn(&DateTime) -> String;

pub struct ImageMetadata {
    pub exif: Exif,
    pub datetime: Option<DateTime>,
}
