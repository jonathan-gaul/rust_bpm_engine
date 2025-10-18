use assert_cmd::Command;
use predicates::prelude::*;

// Run the compiled binary and assert that example processes run and print expected markers.
#[test]
fn runs_examples_and_prints_expected_vars() {
    let mut cmd = Command::cargo_bin("bpm_engine").expect("binary not found");
    let assert = cmd.assert();

    // Expect the output to contain the Example 4 header and the 'Final variables' block
    assert
        .success()
        .stdout(predicate::str::contains("--- Example 4: Command Task with Variables ---"))
        .stdout(predicate::str::contains("Final variables:"));
}
