use common::{WorkerTestConfig, WorkerTestProject};
use uuid::Uuid;

use crate::common::PidError;

mod common;

#[test]
fn test_start_project_already_running() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Three;

    let project_name = worker.project_name(&project);

    let mut cmd = worker.start(&[&project_name]);
    cmd.assert().success();

    let mut cmd = worker.start(&[&project_name]);
    cmd.assert()
        .stderr(format!("{} is already running\n", &project_name));

    assert_eq!(worker.pids(&project_name).unwrap().len(), 1);
}

#[test]
fn test_start_unknown_project() {
    let worker = WorkerTestConfig::new();
    let project = worker.project_name(&WorkerTestProject::Unknown);

    let mut cmd = worker.start(&[&project]);
    cmd.assert().failure();
}

#[test]
fn test_start_success() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::One;

    let project_name = worker.project_name(&project);

    let mut cmd = worker.start(&[&project_name]);
    cmd.assert().success();

    assert_eq!(worker.pids(&project_name).unwrap().len(), 1);
}

#[test]
fn test_start_multiple_success() {
    let worker = WorkerTestConfig::new();
    let project1 = WorkerTestProject::One;
    let project2 = WorkerTestProject::Two;

    let project1_name = worker.project_name(&project1);
    let project2_name = worker.project_name(&project2);

    let mut cmd = worker.start(&[&project1_name, &project2_name]);
    cmd.assert().success();

    assert_eq!(worker.pids(&project1_name).unwrap().len(), 1);
    assert_eq!(worker.pids(&project2_name).unwrap().len(), 1);
}

#[test]
fn test_start_multiple_one_already_running() {
    let worker = WorkerTestConfig::new();
    let project1 = WorkerTestProject::One;
    let project2 = WorkerTestProject::Two;

    let project1_name = worker.project_name(&project1);
    let project2_name = worker.project_name(&project2);

    let mut cmd = worker.start(&[&project1_name]);
    cmd.assert().success();

    let pid1 = worker.pids(&project1_name).unwrap()[0];

    let mut cmd = worker.start(&[&project1_name, &project2_name]);
    cmd.assert().success();

    // Should not start the already running project
    let new_pids1 = worker.pids(&project1_name).unwrap();
    assert_eq!(pid1, new_pids1[0]);
    assert_eq!(new_pids1.len(), 1);

    // Verify that project 2 is running
    assert_eq!(worker.pids(&project2_name).unwrap().len(), 1);
}

#[test]
fn test_start_group_success() {
    let worker = WorkerTestConfig::new();
    let group1 = WorkerTestProject::GroupOne;

    let group1_name = worker.project_name(&group1);

    let mut cmd = worker.start(&[&group1_name]);
    cmd.assert().success();

    let projects = worker.group_projects(&group1);
    let project1_name = worker.project_name(&projects[0]);
    let project2_name = worker.project_name(&projects[1]);

    assert_eq!(worker.pids(&project1_name).unwrap().len(), 1);
    assert_eq!(worker.pids(&project2_name).unwrap().len(), 1);
}

#[test]
fn test_start_group_and_project_success() {
    let worker = WorkerTestConfig::new();
    let group1 = WorkerTestProject::GroupOne;
    let project3 = WorkerTestProject::Three;

    let group1_name = worker.project_name(&group1);
    let project3_name = worker.project_name(&project3);

    let mut cmd = worker.start(&[&group1_name, &project3_name]);
    cmd.assert().success();

    let projects = worker.group_projects(&group1);

    let project1_name = worker.project_name(&projects[0]);
    let project2_name = worker.project_name(&projects[1]);

    assert_eq!(worker.pids(&project1_name).unwrap().len(), 1);
    assert_eq!(worker.pids(&project2_name).unwrap().len(), 1);
    assert_eq!(worker.pids(&project3_name).unwrap().len(), 1);
}

#[test]
fn test_start_multiple_groups() {
    let worker = WorkerTestConfig::new();
    let group1 = WorkerTestProject::GroupOne;
    let group2 = WorkerTestProject::GroupTwo;

    let group1_name = worker.project_name(&group1);
    let group2_name = worker.project_name(&group2);

    let mut cmd = worker.start(&[&group1_name, &group2_name]);
    cmd.assert().success();

    let projects1 = worker.group_projects(&group1);
    let projects2 = worker.group_projects(&group1);

    let project11_name = worker.project_name(&projects1[0]);
    let project12_name = worker.project_name(&projects1[1]);
    let project21_name = worker.project_name(&projects2[0]);
    let project22_name = worker.project_name(&projects2[1]);

    assert_eq!(worker.pids(&project11_name).unwrap().len(), 1);
    assert_eq!(worker.pids(&project12_name).unwrap().len(), 1);
    assert_eq!(worker.pids(&project21_name).unwrap().len(), 1);
    assert_eq!(worker.pids(&project22_name).unwrap().len(), 1);
}

#[test]
fn test_start_starts_dependencies() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Five;

    let dep1 = WorkerTestProject::One;
    let dep2 = WorkerTestProject::Two;

    let project_name = worker.project_name(&project);
    let dep1_name = worker.project_name(&dep1);
    let dep2_name = worker.project_name(&dep2);

    let mut cmd = worker.start(&[&project_name]);
    cmd.assert().success();

    assert_eq!(worker.pids(&project_name).unwrap().len(), 1);
    assert_eq!(worker.pids(&dep1_name).unwrap().len(), 1);
    assert_eq!(worker.pids(&dep2_name).unwrap().len(), 1);
}

#[test]
fn test_start_command_success() {
    let worker = WorkerTestConfig::new();

    let uuid = Uuid::new_v4();
    let echo_cmd = format!("echo 'Hello from {}!' && sleep 5", uuid);

    let mut cmd = worker.start(&["-n", &uuid.to_string(), "-c", &echo_cmd]);
    cmd.assert().success();

    assert_eq!(worker.pids(&uuid.to_string()).unwrap().len(), 1);
}

#[test]
fn test_start_command_require_name() {
    let worker = WorkerTestConfig::new();

    let uuid = Uuid::new_v4();
    let echo_cmd = format!("echo 'Hello from {}!' && sleep 5", uuid);

    let mut cmd = worker.start(&["-c", &echo_cmd]);
    cmd.assert().failure();

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&uuid.to_string()));
}
