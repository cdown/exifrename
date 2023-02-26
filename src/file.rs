use std::ffi::CString;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tempfile::NamedTempFile;

pub fn rename(from: &Path, to: &Path, overwrite: bool) -> io::Result<()> {
    let from_c = CString::new(from.as_os_str().as_bytes()).expect("invalid rename source");
    let to_c = CString::new(to.as_os_str().as_bytes()).expect("invalid rename dest");
    let flags = if overwrite { 0 } else { libc::RENAME_NOREPLACE };

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

pub fn copy_creating_dirs(from: &Path, to_raw: impl Into<PathBuf>, overwrite: bool) -> Result<()> {
    let to = to_raw.into();
    let to_parent = to.parent().context("refusing to move to filesystem root")?;
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
    let ren = rename(from, &to, overwrite);
    if let Err(ref err) = ren {
        if let Some(os_err) = err.raw_os_error() {
            if os_err == libc::EXDEV {
                let tmp_path = NamedTempFile::new_in(to_parent)?.into_temp_path();
                fs::copy(from, &tmp_path)?;
                let res = rename(&tmp_path, &to, overwrite);
                match res {
                    Ok(_) => fs::remove_file(from)?,
                    Err(_) => {
                        fs::remove_file(tmp_path)?;
                        res?;
                    }
                }
            } else {
                ren?;
            }
        } else {
            ren?;
        }
    }
    Ok(())
}
