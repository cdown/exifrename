use exif::{DateTime, Exif};

type FormatterCallback = fn(&ImageMetadata) -> Option<String>;
type DatetimeCallback = fn(&DateTime) -> String;

struct ImageMetadata {
    exif: Exif,
    datetime: Option<DateTime>,
}
