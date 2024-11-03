use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tempfile::NamedTempFile;

#[cfg(target_family = "unix")]
use libc::EXDEV as xdev_err;
#[cfg(target_family = "windows")]
use winapi::shared::winerror::ERROR_NOT_SAME_DEVICE as xdev_err;

#[cfg(target_os = "linux")]
fn rename(from: &Path, to: &Path, overwrite: bool) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let from_c = CString::new(from.as_os_str().as_bytes())?;
    let to_c = CString::new(to.as_os_str().as_bytes())?;
    let flags = if overwrite { 0 } else { libc::RENAME_NOREPLACE };

    // SAFETY: Simple FFI
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

#[cfg(target_os = "macos")]
fn rename(from: &Path, to: &Path, overwrite: bool) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let from_c = CString::new(from.as_os_str().as_bytes())?;
    let to_c = CString::new(to.as_os_str().as_bytes())?;
    let flags = if overwrite { 0 } else { libc::RENAME_EXCL };

    // SAFETY: Simple FFI
    let ret = unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            from_c.as_ptr(),
            libc::AT_FDCWD,
            to_c.as_ptr(),
            flags
        )
    };

    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}


#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn rename(from: &Path, to: &Path, overwrite: bool) -> io::Result<()> {
    use crate::util::die;
    if !overwrite {
        die!("Overwrite-free rename not implemented on this operating system. Use --overwrite.");
    }
    fs::rename(from, to)
}

pub fn copy_creating_dirs(from: &Path, to_raw: impl Into<PathBuf>, overwrite: bool) -> Result<()> {
    let to = to_raw.into();
    let to_parent = to.parent().context("refusing to copy to filesystem root")?;
    fs::create_dir_all(to_parent)?;
    let tmp_path = NamedTempFile::new_in(to_parent)?.into_temp_path();
    fs::copy(from, &tmp_path)?;
    let res = rename(&tmp_path, &to, overwrite);
    if res.is_err() {
        fs::remove_file(tmp_path)?;
        res?;
    }
    Ok(())
}

pub fn rename_creating_dirs(
    from: &Path,
    to_raw: impl Into<PathBuf>,
    overwrite: bool,
) -> Result<()> {
    let to = to_raw.into();
    let to_parent = to.parent().context("refusing to move to filesystem root")?;
    fs::create_dir_all(to_parent)?;

    // Trying to rename cross device? Just copy and unlink the old one
    let ren_samedev = rename(from, &to, overwrite);

    if let Err(ref err) = ren_samedev {
        #[allow(clippy::useless_conversion)] // Necessary for Windows only
        let xdev_err_cast = xdev_err.try_into()?;
        if err.raw_os_error() == Some(xdev_err_cast) {
            copy_creating_dirs(from, &to, overwrite)?;
            fs::remove_file(from)?;
        } else {
            ren_samedev?;
        }
    }
    Ok(())
}
