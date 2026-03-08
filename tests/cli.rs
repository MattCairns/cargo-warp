use std::process::Command;

#[test]
fn test_help_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-warp"))
        .arg("--help")
        .output()
        .expect("Expected --help execution to succeed");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("warp"));
}

#[test]
fn test_invalid_subcommand_fails() {
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-warp"))
        .arg("not-a-command")
        .output()
        .expect("Expected invalid subcommand execution to complete");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand"));
}
