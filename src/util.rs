use std::process;

macro_rules! die {
    ($($arg:tt)*) => {{
        eprintln!("fatal: {}", format!($($arg)*));
        process::exit(1);
    }}
}
