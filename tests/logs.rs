use std::time::Duration;

use common::{WorkerTestConfig, WorkerTestProject};
use uuid::Uuid;

mod common;

#[test]
fn test_logs_project_not_running() {
    let worker = WorkerTestConfig::new();

    let project_name = worker.project_name(&WorkerTestProject::One);

    let mut cmd = worker.logs(&[&project_name]);
    cmd.assert().failure();
}

#[test]
fn test_logs_success() {
    let worker = WorkerTestConfig::new();

    let project_name = worker.project_name(&WorkerTestProject::One);

    let mut cmd = worker.start(&[&project_name]);
    cmd.assert().success();

    // Give the process time to produce output
    std::thread::sleep(Duration::from_millis(500));

    let mut cmd = worker.logs(&[&project_name]);
    cmd.timeout(Duration::from_secs(2));
    let output = cmd.output().expect("Failed to run logs");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Hello from mock!"),
        "Expected output in logs, got: {}",
        stdout
    );
}

#[test]
fn test_logs_command_success() {
    let worker = WorkerTestConfig::new();

    let uuid = Uuid::new_v4();
    let echo_cmd = format!("echo 'Hello from {}!' && sleep 5", uuid);

    let mut cmd = worker.start(&["-n", &uuid.to_string(), "-c", &echo_cmd]);
    cmd.assert().success();

    // Give the process time to produce output
    std::thread::sleep(Duration::from_millis(500));

    let mut cmd = worker.logs(&[&uuid.to_string()]);
    cmd.timeout(Duration::from_secs(2));
    let output = cmd.output().expect("Failed to run logs");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains(&format!("Hello from {}!", uuid)),
        "Expected output in logs, got: {}",
        stdout
    );
}
