#[allow(unused_macros)] // Might only be used on Windows
macro_rules! die {
    ($($arg:tt)*) => {{
        use std::panic;
        eprintln!("fatal: {}", format!($($arg)*));
        panic::set_hook(Box::new(|_| {}));
        panic!(); // Don't use exit() because it does not run destructors
    }}
}
#[allow(unused_imports)]
pub(crate) use die;

pub fn get_usize_len(value_: usize) -> usize {
    let mut len: usize = 1;
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
