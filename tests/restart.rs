use common::{WorkerTestConfig, WorkerTestProject};
use uuid::Uuid;

use crate::common::PidError;

mod common;

#[test]
fn test_restart_unknown_project() {
    let worker = WorkerTestConfig::new();

    let project_name = worker.project_name(&WorkerTestProject::Unknown);

    let mut cmd = worker.restart(&[&project_name]);
    cmd.assert().stdout(format!(
        "{project_name} is not a project nor a running command\n"
    ));
}

#[test]
fn test_restart_success() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::One;

    let project_name = worker.project_name(&project);

    let mut cmd = worker.start(&[&project_name]);
    cmd.assert().success();

    let pid = worker.pids(&project_name).unwrap()[0];

    let mut cmd = worker.restart(&[&project_name]);
    cmd.assert().success();

    let new_pid = worker.pids(&project_name).unwrap()[0];

    assert_ne!(pid, new_pid);
}

#[test]
fn test_restart_multiple_success() {
    let worker = WorkerTestConfig::new();
    let project1 = WorkerTestProject::One;
    let project2 = WorkerTestProject::Two;

    let project1_name = worker.project_name(&project1);
    let project2_name = worker.project_name(&project2);

    let mut cmd = worker.start(&[&project1_name, &project2_name]);
    cmd.assert().success();

    let pid1 = worker.pids(&project1_name).unwrap()[0];
    let pid2 = worker.pids(&project2_name).unwrap()[0];

    let mut cmd = worker.restart(&[&project1_name, &project2_name]);
    cmd.assert().success();

    let new_pid1 = worker.pids(&project1_name).unwrap()[0];
    let new_pid2 = worker.pids(&project2_name).unwrap()[0];

    assert_ne!(pid1, new_pid1);
    assert_ne!(pid2, new_pid2);
}

#[test]
fn test_restart_multiple_only_one_running() {
    let worker = WorkerTestConfig::new();
    let project1 = WorkerTestProject::One;
    let project2 = WorkerTestProject::Two;

    let project1_name = worker.project_name(&project1);
    let project2_name = worker.project_name(&project2);

    let mut cmd = worker.start(&[&project1_name]);
    cmd.assert().success();

    let pid1 = worker.pids(&project1_name).unwrap()[0];

    let mut cmd = worker.restart(&[&project1_name, &project2_name]);
    cmd.assert().success();

    let new_pid1 = worker.pids(&project1_name).unwrap()[0];
    assert_ne!(pid1, new_pid1);

    // Verify that the project that wasn't running is not started
    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project2_name));
}

#[test]
fn test_restart_group_success() {
    let worker = WorkerTestConfig::new();
    let group = WorkerTestProject::GroupOne;

    let group_name = worker.project_name(&group);

    let mut cmd = worker.start(&[&group_name]);
    cmd.assert().success();

    let projects = worker.group_projects(&group);

    let project1_name = worker.project_name(&projects[0]);
    let project2_name = worker.project_name(&projects[1]);

    let pid1 = worker.pids(&project1_name).unwrap()[0];
    let pid2 = worker.pids(&project2_name).unwrap()[0];

    let mut cmd = worker.restart(&[&group_name]);
    cmd.assert().success();

    let new_pid1 = worker.pids(&project1_name).unwrap()[0];
    let new_pid2 = worker.pids(&project2_name).unwrap()[0];

    assert_ne!(pid1, new_pid1);
    assert_ne!(pid2, new_pid2);
}

#[test]
fn test_restart_multiple_group_success() {
    let worker = WorkerTestConfig::new();
    let group1 = WorkerTestProject::GroupOne;
    let group2 = WorkerTestProject::GroupTwo;

    let group1_name = worker.project_name(&group1);
    let group2_name = worker.project_name(&group2);

    let mut cmd = worker.start(&[&group1_name, &group2_name]);
    cmd.assert().success();

    let projects1 = worker.group_projects(&group1);
    let projects2 = worker.group_projects(&group2);

    let project11_name = worker.project_name(&projects1[0]);
    let project12_name = worker.project_name(&projects1[1]);
    let project21_name = worker.project_name(&projects2[0]);
    let project22_name = worker.project_name(&projects2[1]);

    let pid1 = worker.pids(&project11_name).unwrap()[0];
    let pid2 = worker.pids(&project12_name).unwrap()[0];
    let pid3 = worker.pids(&project21_name).unwrap()[0];
    let pid4 = worker.pids(&project22_name).unwrap()[0];

    let mut cmd = worker.restart(&[&group1_name, &group2_name]);
    cmd.assert().success();

    let new_pid1 = worker.pids(&project11_name).unwrap()[0];
    let new_pid2 = worker.pids(&project12_name).unwrap()[0];
    let new_pid3 = worker.pids(&project21_name).unwrap()[0];
    let new_pid4 = worker.pids(&project22_name).unwrap()[0];

    assert_ne!(pid1, new_pid1);
    assert_ne!(pid2, new_pid2);
    assert_ne!(pid3, new_pid3);
    assert_ne!(pid4, new_pid4);
}

#[test]
fn test_restart_group_one_project_running() {
    let worker = WorkerTestConfig::new();
    let group1 = WorkerTestProject::GroupOne;
    let projects = worker.group_projects(&group1);

    let project1_name = worker.project_name(&projects[0]);
    let project2_name = worker.project_name(&projects[1]);
    let group1_name = worker.project_name(&group1);

    let mut cmd = worker.start(&[&project1_name]);
    cmd.assert().success();

    let pid1 = worker.pids(&project1_name).unwrap()[0];

    let mut cmd = worker.restart(&[&group1_name]);
    cmd.assert().success();

    let new_pid1 = worker.pids(&project1_name).unwrap()[0];
    assert_ne!(pid1, new_pid1);

    // Verify that the project that wasn't running is not started
    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project2_name));
}

#[test]
fn test_restart_not_restarting_dependencies() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Five;

    let dep1 = WorkerTestProject::One;
    let dep2 = WorkerTestProject::Two;

    let project_name = worker.project_name(&project);
    let dep1_name = worker.project_name(&dep1);
    let dep2_name = worker.project_name(&dep2);

    let mut cmd = worker.start(&[&project_name, &project_name]);
    cmd.assert().success();

    let pid = worker.pids(&project_name).unwrap()[0];
    let pid1 = worker.pids(&dep1_name).unwrap()[0];
    let pid2 = worker.pids(&dep2_name).unwrap()[0];

    let mut cmd = worker.restart(&[&project_name, &project_name]);
    cmd.assert().success();

    let new_pid = worker.pids(&project_name).unwrap()[0];
    let new_pid1 = worker.pids(&dep1_name).unwrap()[0];
    let new_pid2 = worker.pids(&dep2_name).unwrap()[0];

    assert_ne!(pid, new_pid);
    assert_eq!(pid1, new_pid1);
    assert_eq!(pid2, new_pid2);
}

#[test]
fn test_restart_command_success() {
    let worker = WorkerTestConfig::new();

    let uuid = Uuid::new_v4();
    let echo_cmd = format!("echo 'Hello from {}!' && sleep 5", uuid);

    let mut cmd = worker.start(&["-n", &uuid.to_string(), "-c", &echo_cmd]);
    cmd.assert().success();

    let pid = worker.pids(&uuid.to_string()).unwrap()[0];

    let mut cmd = worker.restart(&[&uuid.to_string()]);
    cmd.assert().success();

    let new_pid = worker.pids(&uuid.to_string()).unwrap()[0];
    assert_ne!(pid, new_pid);
}
