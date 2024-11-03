use anyhow::Result;

#[cfg(any(target_os = "linux", target_family = "windows"))] // Others must use --overwrite
#[test]
fn test_rename_no_overwrite() -> Result<()> {
    // Set up test environment
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();

    std::fs::copy("tests/data/1.jpg", temp_path.join("1.jpg"))?;

    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(temp_path)
        .arg("-f")
        .arg("{year}_{day}")
        .current_dir(temp_path)
        .assert()
        .success();

    let expected_name = "2023_01.jpg";
    assert!(temp_path.join(expected_name).exists());

    // No --overwrite, so should fail
    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(temp_path)
        .arg("-f")
        .arg("{year}_{day}")
        .current_dir(temp_path)
        .assert()
        .failure();

    Ok(())
}

#[test]
fn test_rename_overwrite() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();

    std::fs::copy("tests/data/1.jpg", temp_path.join("1.jpg"))?;

    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(temp_path)
        .arg("--overwrite")
        .arg("-f")
        .arg("{year}_{day}")
        .current_dir(temp_path)
        .assert()
        .success();

    // With --overwrite it should succeed
    let mut cmd = assert_cmd::Command::cargo_bin("exifrename")?;
    cmd.arg(temp_path)
        .arg("--overwrite")
        .arg("-f")
        .arg("{year}_{day}")
        .current_dir(temp_path)
        .assert()
        .success();

    Ok(())
}
