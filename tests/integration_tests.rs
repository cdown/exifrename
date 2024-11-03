use anyhow::Result;

#[cfg(any(target_os = "linux", target_family = "windows"))] // Others must use --overwrite
#[test]
fn test_rename_no_overwrite() -> Result<()> {
    // Set up test environment
    let from_dir = tempfile::tempdir()?;
    let from_path = from_dir.path();

    let to_dir = tempfile::tempdir()?;
    let to_path = to_dir.path();

    std::fs::copy("tests/data/1.jpg", from_path.join("1.jpg"))?;

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

    // No --overwrite, so should fail
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

    std::fs::copy("tests/data/1.jpg", from_path.join("1.jpg"))?;

    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(from_path)
        .arg("--overwrite")
        .arg("--copy")
        .arg("-f")
        .arg("{year}_{day}")
        .current_dir(to_path)
        .assert()
        .success();

    // With --overwrite, it should succeed even when run again
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
