use std::{fs, process::Command};

#[test]
fn init_is_idempotent_and_status_supports_json() {
    let temp = tempfile::tempdir().unwrap();
    let executable = env!("CARGO_BIN_EXE_bam");
    let init = Command::new(executable)
        .current_dir(temp.path())
        .arg("init")
        .output()
        .expect("launch bam init");
    assert!(
        init.status.success(),
        "{:?}",
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(temp.path().join(".bam/state.sqlite").exists());

    let config_path = temp.path().join(".bam/config.toml");
    let before = fs::read(&config_path).unwrap();

    let second = Command::new(executable)
        .current_dir(temp.path())
        .arg("init")
        .output()
        .expect("launch second bam init");
    assert!(
        second.status.success(),
        "{:?}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(fs::read(&config_path).unwrap(), before);

    let status = Command::new(executable)
        .current_dir(temp.path())
        .args(["status", "--json"])
        .output()
        .expect("launch bam status --json");
    assert!(status.status.success());
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(stdout.contains("\"tasks\":0"));
    assert!(stdout.contains("\"scheduler\":\"NOT_IMPLEMENTED\""));
}

#[test]
fn doctor_fails_closed_when_required_executables_are_missing() {
    let temp = tempfile::tempdir().unwrap();
    let executable = env!("CARGO_BIN_EXE_bam");
    let init = Command::new(executable)
        .current_dir(temp.path())
        .arg("init")
        .output()
        .unwrap();
    assert!(init.status.success());

    let empty_path = temp.path().join("empty-path");
    fs::create_dir_all(&empty_path).unwrap();
    let doctor = Command::new(executable)
        .current_dir(temp.path())
        .env("PATH", &empty_path)
        .arg("doctor")
        .output()
        .expect("launch bam doctor");
    assert!(!doctor.status.success());
    let stderr = String::from_utf8_lossy(&doctor.stderr);
    assert!(stderr.contains("missing required executables"));
    assert!(!stderr.contains("cargo"));
}

#[test]
fn human_status_remains_available() {
    let temp = tempfile::tempdir().unwrap();
    let executable = env!("CARGO_BIN_EXE_bam");
    assert!(Command::new(executable)
        .current_dir(temp.path())
        .arg("init")
        .status()
        .unwrap()
        .success());

    let status = Command::new(executable)
        .current_dir(temp.path())
        .arg("status")
        .output()
        .unwrap();
    assert!(status.status.success());
    assert!(String::from_utf8_lossy(&status.stdout).contains("tasks: 0"));
}

#[test]
fn unknown_commands_fail() {
    let output = Command::new(env!("CARGO_BIN_EXE_bam"))
        .arg("run")
        .output()
        .unwrap();
    assert!(!output.status.success(), "scheduler is not yet implemented");
}
