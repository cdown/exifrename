use anyhow::Result;
use predicates::prelude::*;
use std::fs;

#[cfg(any(target_os = "linux", target_family = "windows", target_os = "macos"))] // Others must use --overwrite
#[test]
fn test_rename_no_overwrite() -> Result<()> {
    let from_dir = tempfile::tempdir()?;
    let from_path = from_dir.path();
    let to_dir = tempfile::tempdir()?;
    let to_path = to_dir.path();

    fs::copy("tests/data/1.jpg", from_path.join("1.jpg"))?;

    // First run: should succeed
    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(from_path)
        .arg("--copy")
        .arg("-f")
        .arg("{year}_{day}")
        .current_dir(to_path)
        .assert()
        .success();

    let expected_name = "2023_01.jpg";
    assert!(to_path.join(expected_name).exists());

    // Second run without overwrite: should fail
    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(from_path)
        .arg("--copy")
        .arg("-f")
        .arg("{year}_{day}")
        .current_dir(to_path)
        .assert()
        .failure();

    Ok(())
}

#[test]
fn test_rename_overwrite() -> Result<()> {
    let from_dir = tempfile::tempdir()?;
    let from_path = from_dir.path();
    let to_dir = tempfile::tempdir()?;
    let to_path = to_dir.path();

    fs::copy("tests/data/1.jpg", from_path.join("1.jpg"))?;

    // First run with overwrite: should succeed
    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(from_path)
        .arg("--overwrite")
        .arg("--copy")
        .arg("-f")
        .arg("{year}_{day}")
        .current_dir(to_path)
        .assert()
        .success();

    // Second run with overwrite: should also succeed
    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(from_path)
        .arg("--overwrite")
        .arg("--copy")
        .arg("-f")
        .arg("{year}_{day}")
        .current_dir(to_path)
        .assert()
        .success();

    Ok(())
}

#[test]
fn test_dry_run() -> Result<()> {
    let from_dir = tempfile::tempdir()?;
    let from_path = from_dir.path();
    let to_dir = tempfile::tempdir()?;
    let to_path = to_dir.path();

    fs::copy("tests/data/1.jpg", from_path.join("1.jpg"))?;

    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(from_path)
        .arg("--dry-run")
        .arg("--copy")
        .arg("-f")
        .arg("{year}_{month}_{day}")
        .current_dir(to_path)
        .assert()
        .success();

    // Check that the file was not copied
    let expected_name = "2023_01_01.jpg";
    assert!(!to_path.join(expected_name).exists());

    Ok(())
}

#[test]
fn test_no_counter() -> Result<()> {
    let from_dir = tempfile::tempdir()?;
    let from_path = from_dir.path();
    let to_dir = tempfile::tempdir()?;
    let to_path = to_dir.path();

    fs::copy("tests/data/1.jpg", from_path.join("1.jpg"))?;
    fs::copy("tests/data/1.jpg", from_path.join("2.jpg"))?;

    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(from_path)
        .arg("--no-counter")
        .arg("--copy")
        .arg("-f")
        .arg("{year}_{month}_{day}")
        .current_dir(to_path)
        .assert()
        .failure();

    Ok(())
}

#[test]
fn test_no_counter_allow_overwrite() -> Result<()> {
    // Test that with --no-counter, duplicate filenames cause an error
    let from_dir = tempfile::tempdir()?;
    let from_path = from_dir.path();
    let to_dir = tempfile::tempdir()?;
    let to_path = to_dir.path();

    fs::copy("tests/data/1.jpg", from_path.join("1.jpg"))?;
    fs::copy("tests/data/1.jpg", from_path.join("2.jpg"))?;

    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(from_path)
        .arg("--overwrite")
        .arg("--no-counter")
        .arg("--copy")
        .arg("-f")
        .arg("{year}_{month}_{day}")
        .current_dir(to_path)
        .assert()
        .success();

    Ok(())
}

#[test]
fn test_missing_exif() -> Result<()> {
    let from_dir = tempfile::tempdir()?;
    let from_path = from_dir.path();
    let to_dir = tempfile::tempdir()?;
    let to_path = to_dir.path();

    fs::copy("tests/data/noexif.jpg", from_path.join("noexif.jpg"))?;

    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(from_path)
        .arg("--copy")
        .arg("-f")
        .arg("{year}")
        .current_dir(to_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to get new name"));

    Ok(())
}
