use common::{WorkerTestConfig, WorkerTestProject};
use uuid::Uuid;

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

    assert!(worker.state_file(&project_name).is_none());
    assert_eq!(worker.pids(project).len(), 0);
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

    assert!(worker.state_file(&project2_name).is_none());
    assert_eq!(worker.pids(project2).len(), 0);

    // Assert that project 3 is still running
    assert!(worker.state_file(&project3_name).is_some());
    assert_eq!(worker.pids(project3).len(), 1);
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

    assert!(worker.state_file(&project2_name).is_none());
    assert_eq!(worker.pids(project2).len(), 0);

    assert!(worker.state_file(&project3_name).is_none());
    assert_eq!(worker.pids(project3).len(), 0);
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

    assert!(worker.state_file(&project2_name).is_none());
    assert_eq!(worker.pids(project2).len(), 0);

    // Stop the projects
    let mut cmd = worker.stop(&[&project2_name, &project3_name]);
    cmd.assert().success();

    // Assert that the project is still stopped
    assert!(worker.state_file(&project2_name).is_none());
    assert_eq!(worker.pids(project2).len(), 0);

    assert!(worker.state_file(&project3_name).is_none());
    assert_eq!(worker.pids(project3).len(), 0);
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

    // Verify that the state file exists
    assert!(worker.state_file(&project1_name).is_none());
    assert!(worker.state_file(&project2_name).is_none());
    assert_eq!(worker.pids(projects[0]).len(), 0);
    assert_eq!(worker.pids(projects[1]).len(), 0);
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

    // Verify that the state file exists
    assert!(worker.state_file(&project11_name).is_none());
    assert!(worker.state_file(&project12_name).is_none());
    assert!(worker.state_file(&project21_name).is_none());
    assert!(worker.state_file(&project22_name).is_none());
    assert_eq!(worker.pids(projects1[0]).len(), 0);
    assert_eq!(worker.pids(projects1[1]).len(), 0);
    assert_eq!(worker.pids(projects2[0]).len(), 0);
    assert_eq!(worker.pids(projects2[1]).len(), 0);
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

    // Verify that the state file exists
    assert!(worker.state_file(&project1_name).is_none());
    assert!(worker.state_file(&project2_name).is_none());
    assert!(worker.state_file(&project3_name).is_none());
    assert_eq!(worker.pids(projects1[0]).len(), 0);
    assert_eq!(worker.pids(projects1[1]).len(), 0);
    assert_eq!(worker.pids(project3).len(), 0);
}

#[test]
fn test_stop_not_stop_dependencies() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Five;
    let dep1 = WorkerTestProject::One;
    let dep2 = WorkerTestProject::Two;

    let project_name = worker.project_name(&project);

    // Start the project
    let mut cmd = worker.start(&[&project_name]);
    cmd.assert().success();

    let mut cmd = worker.stop(&[&project_name]);
    cmd.assert().success();

    // Verify that the state file exists
    assert_eq!(worker.pids(project).len(), 0);
    assert_eq!(worker.pids(dep1).len(), 1);
    assert_eq!(worker.pids(dep2).len(), 1);
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

    assert!(worker.state_file(&uuid.to_string()).is_none());
    assert_eq!(worker.cmd_pids(&echo_cmd).len(), 0);
}
