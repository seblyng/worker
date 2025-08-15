use common::{WorkerTestConfig, WorkerTestProject};
use uuid::Uuid;

use crate::common::PidError;

mod common;

#[test]
fn test_run_project() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Four;

    let project_name = worker.project_name(&project);

    let mut cmd = worker.run(&[&project_name]);
    cmd.assert().success().stdout("Hello from test!\n");

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project_name));
}

#[test]
fn test_run_project_starts_dependencies() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Six;

    let dep1 = WorkerTestProject::One;
    let dep2 = WorkerTestProject::Two;

    let project_name = worker.project_name(&project);
    let dep1_name = worker.project_name(&dep1);
    let dep2_name = worker.project_name(&dep2);

    let mut cmd = worker.run(&[&project_name]);
    cmd.assert().stdout("Hello from test!\n");

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project_name));
    assert_eq!(worker.pids(&dep1_name).unwrap().len(), 1);
    assert_eq!(worker.pids(&dep2_name).unwrap().len(), 1);
}

#[test]
fn test_run_command_success() {
    let worker = WorkerTestConfig::new();

    let uuid = Uuid::new_v4();
    let echo_cmd = format!("echo 'Hello from {}!'", uuid);

    let mut cmd = worker.run(&["-n", &uuid.to_string(), "-c", &echo_cmd]);
    cmd.assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&uuid.to_string()));
}

#[test]
fn test_run_command_require_name() {
    let worker = WorkerTestConfig::new();

    let uuid = Uuid::new_v4();
    let echo_cmd = format!("echo 'Hello from {}!'", uuid);

    let mut cmd = worker.start(&["-c", &echo_cmd]);
    cmd.assert().failure();

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&uuid.to_string()));
}
