macro_rules! die {
    ($($arg:tt)*) => {{
        use std::process;
        eprintln!("fatal: {}", format!($($arg)*));
        process::exit(1);
    }}
}
pub(crate) use die;
