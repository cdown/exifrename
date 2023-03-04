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

pub const fn get_usize_len(value_: usize) -> usize {
    let mut len = 1;
    let mut value = value_;
    if value == 0 {
        return 0;
    }
    while value > 9 {
        len += 1;
        value /= 10;
    }
    len
}
