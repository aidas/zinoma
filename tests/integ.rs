use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn circular_dependency() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("zinoma")?;
    cmd.arg("-p")
        .arg("tests/integ/circular_dependency")
        .arg("target_1");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Circular dependency"));

    Ok(())
}
