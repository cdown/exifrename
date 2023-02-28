#[allow(unused_macros)] // Might only be used on Windows
macro_rules! die {
    ($($arg:tt)*) => {{
        use std::process;
        eprintln!("fatal: {}", format!($($arg)*));
        process::exit(1);
    }}
}
#[allow(unused_imports)]
pub(crate) use die;
