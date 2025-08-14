use std::time::{Duration, Instant};

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

    let timeout = Duration::new(1, 0);
    let start = Instant::now();

    // Try multiple times since it may not output immediately
    while Instant::now().duration_since(start) < timeout {
        let mut cmd = worker.logs(&[&project_name]);
        cmd.assert().success();

        let output = &cmd.output().unwrap().stdout;
        let stdout = std::str::from_utf8(output).unwrap();
        if stdout.contains("Hello from mock!") {
            return;
        }
    }
    unreachable!("Couldn't find output in 1 second")
}

#[test]
fn test_logs_command_success() {
    let worker = WorkerTestConfig::new();

    let uuid = Uuid::new_v4();
    let echo_cmd = format!("echo 'Hello from {}!' && sleep 5", uuid);

    let mut cmd = worker.start(&["-n", &uuid.to_string(), "-c", &echo_cmd]);
    cmd.assert().success();

    let timeout = Duration::new(1, 0);
    let start = Instant::now();

    // Try multiple times since it may not output immediately
    while Instant::now().duration_since(start) < timeout {
        let mut cmd = worker.logs(&[&uuid.to_string()]);
        cmd.assert().success();

        let output = &cmd.output().unwrap().stdout;
        let stdout = std::str::from_utf8(output).unwrap();
        println!("stdout: {}", stdout);
        if stdout.contains(&format!("Hello from {}!", uuid)) {
            return;
        }
    }
    unreachable!("Couldn't find output in 1 second")
}
