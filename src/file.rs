use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tempfile::NamedTempFile;

#[cfg(target_family = "unix")]
use libc::EXDEV as xdev_err;
#[cfg(target_family = "windows")]
use windows_sys::Win32::Foundation::ERROR_NOT_SAME_DEVICE as xdev_err;

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

#[cfg(not(any(target_os = "linux", target_family = "windows")))]
fn rename(from: &Path, to: &Path, overwrite: bool) -> io::Result<()> {
    use crate::util::die;
    if !overwrite {
        die!("overwrite-free rename not implemented for non-Linux");
    }
    fs::rename(from, to)
}

#[cfg(target_family = "windows")]
fn rename(from: &Path, to: &Path, overwrite: bool) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Storage::FileSystem::{GetFileAttributesW, MoveFileExW, MOVEFILE_REPLACE_EXISTING};
    use std::{iter, ptr};

    let from_wide: Vec<u16> = from
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let to_wide: Vec<u16> = to
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();

    let flags = if overwrite {
        MOVEFILE_REPLACE_EXISTING
    } else {
        0
    };

    // Initial debug output
    dbg!(from);
    dbg!(to);
    dbg!(to.exists(), overwrite, flags);

    // Check file attributes of the destination file `to`
    let to_attributes = unsafe { GetFileAttributesW(to_wide.as_ptr()) };
    dbg!(to_attributes);

    // Additional metadata check before the rename
    dbg!(std::fs::metadata(to));
    
    // Attempt the rename operation
    let ret = unsafe { MoveFileExW(from_wide.as_ptr(), to_wide.as_ptr(), flags) };

    // Recheck existence after attempting the rename
    dbg!(ret);
    dbg!(to.exists());

    if ret == 0 {
        let err = unsafe { GetLastError() };
        Err(io::Error::from_raw_os_error(err as i32))
    } else {
        Ok(())
    }
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
