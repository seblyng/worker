use common::{WorkerTestConfig, WorkerTestProject};
use uuid::Uuid;

use crate::common::PidError;

mod common;

#[test]
fn test_stop_unknown_project() {
    let worker = WorkerTestConfig::new();

    let project_name = worker.project_name(&WorkerTestProject::Unknown);

    let mut cmd = worker.stop(&[&project_name]);
    cmd.assert().stdout(format!(
        "{project_name} is not a project nor a running command\n",
    ));
}

#[test]
fn test_stop_command_not_running() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Two;

    let project_name = worker.project_name(&project);

    let mut cmd = worker.stop(&[&project_name]);
    cmd.assert()
        .stdout(format!("{project_name} is not running\n",));
}

#[test]
fn test_stop_success() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Two;

    let project_name = worker.project_name(&project);

    // Start the project
    let mut cmd = worker.start(&[&project_name]);
    cmd.assert().success();

    // Stop the project
    let mut cmd = worker.stop(&[&project_name]);
    cmd.assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project_name));
}

#[test]
fn test_stop_success_one_still_running() {
    let worker = WorkerTestConfig::new();
    let project2 = WorkerTestProject::Two;
    let project3 = WorkerTestProject::Three;

    let project2_name = worker.project_name(&project2);
    let project3_name = worker.project_name(&project3);

    // Start the project
    let mut cmd = worker.start(&[&project2_name, &project3_name]);
    cmd.assert().success();

    // Stop the project
    let mut cmd = worker.stop(&[&project2_name]);
    cmd.assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project2_name));
    assert_eq!(worker.pids(&project3_name).unwrap().len(), 1);
}

#[test]
fn test_stop_multiple_success() {
    let worker = WorkerTestConfig::new();
    let project2 = WorkerTestProject::Two;
    let project3 = WorkerTestProject::Three;

    let project2_name = worker.project_name(&project2);
    let project3_name = worker.project_name(&project3);

    // Start the projects
    let mut cmd = worker.start(&[&project2_name, &project3_name]);
    cmd.assert().success();

    // Stop the projects
    let mut cmd = worker.stop(&[&project2_name, &project3_name]);
    cmd.assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project2_name));
    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project3_name));
}

#[test]
fn test_stop_multiple_one_already_stopped() {
    let worker = WorkerTestConfig::new();
    let project2 = WorkerTestProject::Two;
    let project3 = WorkerTestProject::Three;

    let project2_name = worker.project_name(&project2);
    let project3_name = worker.project_name(&project3);

    // Start the projects
    let mut cmd = worker.start(&[&project2_name, &project3_name]);
    cmd.assert().success();

    // Stop the projects
    let mut cmd = worker.stop(&[&project2_name]);
    cmd.assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project2_name));

    // Stop the projects
    let mut cmd = worker.stop(&[&project2_name, &project3_name]);
    cmd.assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project2_name));
    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project3_name));
}

#[test]
fn test_stop_group_success() {
    let worker = WorkerTestConfig::new();
    let group1 = WorkerTestProject::GroupOne;

    let group1_name = worker.project_name(&group1);

    // Start the project
    let mut cmd = worker.start(&[&group1_name]);
    cmd.assert().success();

    let mut cmd = worker.stop(&[&group1_name]);
    cmd.assert().success();

    let projects = worker.group_projects(&group1);

    let project1_name = worker.project_name(&projects[0]);
    let project2_name = worker.project_name(&projects[1]);

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project1_name));
    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project2_name));
}

#[test]
fn test_stop_multiple_groups_success() {
    let worker = WorkerTestConfig::new();
    let group1 = WorkerTestProject::GroupOne;
    let group2 = WorkerTestProject::GroupTwo;

    let group1_name = worker.project_name(&group1);
    let group2_name = worker.project_name(&group2);

    // Start the project
    let mut cmd = worker.start(&[&group1_name, &group2_name]);
    cmd.assert().success();

    let mut cmd = worker.stop(&[&group1_name, &group2_name]);
    cmd.assert().success();

    let projects1 = worker.group_projects(&group1);
    let projects2 = worker.group_projects(&group2);

    let project11_name = worker.project_name(&projects1[0]);
    let project12_name = worker.project_name(&projects1[1]);
    let project21_name = worker.project_name(&projects2[0]);
    let project22_name = worker.project_name(&projects2[1]);

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project11_name));
    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project12_name));
    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project21_name));
    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project22_name));
}

#[test]
fn test_stop_groups_and_project_success() {
    let worker = WorkerTestConfig::new();
    let group1 = WorkerTestProject::GroupOne;
    let project3 = WorkerTestProject::Three;

    let group1_name = worker.project_name(&group1);
    let project3_name = worker.project_name(&project3);

    // Start the project
    let mut cmd = worker.start(&[&group1_name, &project3_name]);
    cmd.assert().success();

    let mut cmd = worker.stop(&[&group1_name, &project3_name]);
    cmd.assert().success();

    let projects1 = worker.group_projects(&group1);

    let project1_name = worker.project_name(&projects1[0]);
    let project2_name = worker.project_name(&projects1[1]);

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project1_name));
    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project2_name));
    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project3_name));
}

#[test]
fn test_stop_not_stop_dependencies() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Five;
    let dep1 = WorkerTestProject::One;
    let dep2 = WorkerTestProject::Two;

    let project_name = worker.project_name(&project);
    let dep1_name = worker.project_name(&dep1);
    let dep2_name = worker.project_name(&dep2);

    // Start the project
    let mut cmd = worker.start(&[&project_name]);
    cmd.assert().success();

    let mut cmd = worker.stop(&[&project_name]);
    cmd.assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&project_name));
    assert_eq!(worker.pids(&dep1_name).unwrap().len(), 1);
    assert_eq!(worker.pids(&dep2_name).unwrap().len(), 1);
}

#[test]
fn test_stop_command_success() {
    let worker = WorkerTestConfig::new();

    let uuid = Uuid::new_v4();
    let echo_cmd = format!("echo 'Hello from {}!' && sleep 5", uuid);

    let mut cmd = worker.start(&["-n", &uuid.to_string(), "-c", &echo_cmd]);
    cmd.assert().success();

    // Stop the project
    let mut cmd = worker.stop(&[&uuid.to_string()]);
    cmd.assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids(&uuid.to_string()));
}
