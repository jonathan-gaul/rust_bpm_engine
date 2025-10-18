use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn binary_emits_multiple_vars_and_echo() {
    let mut cmd = Command::cargo_bin("bpm_engine").expect("binary not found");
    let assert = cmd.assert();

    // Example 5 prints 'User Information:' and shows user_id etc.
    assert
        .success()
        .stdout(predicate::str::contains("--- Example 5: Setting Multiple Variables ---"))
        .stdout(predicate::str::contains("User Information:"))
        .stdout(predicate::str::contains("ID:"));
}
