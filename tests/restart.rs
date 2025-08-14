use common::{WorkerTestConfig, WorkerTestProject};
use uuid::Uuid;

mod common;

#[test]
fn test_restart_unknown_project() {
    let worker = WorkerTestConfig::new();

    let project_name = worker.project_name(&WorkerTestProject::Unknown);

    let mut cmd = worker.restart(&[&project_name]);
    cmd.assert().failure();
}

#[test]
fn test_restart_success() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::One;

    let project_name = worker.project_name(&project);

    let mut cmd = worker.start(&[&project_name]);
    cmd.assert().success();

    // Verify that the state file exists
    assert!(worker.state_file(&project_name).is_some());

    let pid = worker.pids(project)[0];

    let mut cmd = worker.restart(&[&project_name]);
    cmd.assert().success();

    // Verify that the state file exists
    assert!(worker.state_file(&project_name).is_some());

    let new_pid = worker.pids(project)[0];

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

    let pid1 = worker.pids(project1)[0];
    let pid2 = worker.pids(project2)[0];

    let mut cmd = worker.restart(&[&project1_name, &project2_name]);
    cmd.assert().success();

    let new_pid1 = worker.pids(project1)[0];
    let new_pid2 = worker.pids(project2)[0];

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

    let pid1 = worker.pids(project1)[0];

    let mut cmd = worker.restart(&[&project1_name, &project2_name]);
    cmd.assert().success();

    let new_pid1 = worker.pids(project1)[0];
    assert_ne!(pid1, new_pid1);

    // Verify that the project that wasn't running is not started
    assert!(worker.state_file(&project2_name).is_none());
    assert_eq!(worker.pids(project2).len(), 0);
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

    assert!(worker.state_file(&project1_name).is_some());
    assert!(worker.state_file(&project2_name).is_some());

    let pid1 = worker.pids(projects[0])[0];
    let pid2 = worker.pids(projects[1])[0];

    let mut cmd = worker.restart(&[&group_name]);
    cmd.assert().success();

    // Verify that the state file exists
    assert!(worker.state_file(&project1_name).is_some());
    assert!(worker.state_file(&project2_name).is_some());

    let new_pid1 = worker.pids(projects[0])[0];
    let new_pid2 = worker.pids(projects[1])[0];

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

    assert!(worker.state_file(&project11_name).is_some());
    assert!(worker.state_file(&project12_name).is_some());

    let pid1 = worker.pids(projects1[0])[0];
    let pid2 = worker.pids(projects1[1])[0];
    let pid3 = worker.pids(projects2[0])[0];
    let pid4 = worker.pids(projects2[1])[0];

    let mut cmd = worker.restart(&[&group1_name, &group2_name]);
    cmd.assert().success();

    // Verify that the state file exists
    assert!(worker.state_file(&project11_name).is_some());
    assert!(worker.state_file(&project11_name).is_some());
    assert!(worker.state_file(&project21_name).is_some());
    assert!(worker.state_file(&project22_name).is_some());

    let new_pid1 = worker.pids(projects1[0])[0];
    let new_pid2 = worker.pids(projects1[1])[0];
    let new_pid3 = worker.pids(projects2[0])[0];
    let new_pid4 = worker.pids(projects2[1])[0];

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

    let pid1 = worker.pids(projects[0])[0];

    let mut cmd = worker.restart(&[&group1_name]);
    cmd.assert().success();

    // Verify that the state file exists
    assert!(worker.state_file(&project1_name).is_some());

    let new_pid1 = worker.pids(projects[0])[0];
    assert_ne!(pid1, new_pid1);

    // Verify that the project that wasn't running is not started
    assert!(worker.state_file(&project2_name).is_none());
    assert_eq!(worker.pids(projects[1]).len(), 0);
}

#[test]
fn test_restart_not_restarting_dependencies() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Five;

    let dep1 = WorkerTestProject::One;
    let dep2 = WorkerTestProject::Two;

    let project_name = worker.project_name(&project);

    let mut cmd = worker.start(&[&project_name, &project_name]);
    cmd.assert().success();

    let pid = worker.pids(project)[0];
    let pid1 = worker.pids(dep1)[0];
    let pid2 = worker.pids(dep2)[0];

    let mut cmd = worker.restart(&[&project_name, &project_name]);
    cmd.assert().success();

    let new_pid = worker.pids(project)[0];
    let new_pid1 = worker.pids(dep1)[0];
    let new_pid2 = worker.pids(dep2)[0];

    assert_ne!(pid, new_pid);
    assert_eq!(pid1, new_pid1);
    assert_eq!(pid2, new_pid2);
}

// TODO(seb): Restart is currently not working with a one-off command
// Not registered as a project in the config
#[test]
fn test_restart_command_success() {
    let worker = WorkerTestConfig::new();

    let uuid = Uuid::new_v4();
    let echo_cmd = format!("echo 'Hello from {}!' && sleep 5", uuid);

    let mut cmd = worker.start(&["-n", &uuid.to_string(), "-c", &echo_cmd]);
    cmd.assert().success();

    // Verify that the state file exists
    assert!(worker.state_file(&uuid.to_string()).is_some());

    let pid = worker.cmd_pids(&echo_cmd)[0];

    let mut cmd = worker.restart(&[&uuid.to_string()]);
    cmd.assert().success();

    // Verify that the state file exists
    assert!(worker.state_file(&uuid.to_string()).is_some());

    let new_pid = worker.cmd_pids(&echo_cmd)[0];

    assert_ne!(pid, new_pid);
}
