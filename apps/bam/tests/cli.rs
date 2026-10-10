use std::process::Command;

#[test]
fn init_config_and_status_are_real_commands() {
    let temp = tempfile::tempdir().unwrap();
    let executable = env!("CARGO_BIN_EXE_bam");
    let init = Command::new(executable)
        .current_dir(temp.path())
        .arg("init")
        .output()
        .expect("launch bam init");
    assert!(init.status.success(), "{:?}", String::from_utf8_lossy(&init.stderr));
    assert!(temp.path().join(".bam/state.sqlite").exists());

    let status = Command::new(executable)
        .current_dir(temp.path())
        .arg("status")
        .output()
        .expect("launch bam status");
    assert!(status.status.success());
    assert!(String::from_utf8_lossy(&status.stdout).contains("tasks: 0"));

    let second = Command::new(executable)
        .current_dir(temp.path())
        .arg("init")
        .output().unwrap();
    assert!(!second.status.success(), "must not overwrite user config");
}

#[test]
fn unknown_commands_fail() {
    let output = Command::new(env!("CARGO_BIN_EXE_bam"))
        .arg("run")
        .output().unwrap();
    assert!(!output.status.success(), "scheduler is not yet implemented");
}
