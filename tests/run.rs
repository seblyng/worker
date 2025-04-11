use common::{WorkerTestConfig, WorkerTestProject};

mod common;

#[test]
fn test_run_project() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Four;

    let mut cmd = worker.run(project);
    cmd.assert().success().stdout("Hello from test!\n");

    assert_eq!(worker.pids(project).len(), 0);
}

#[test]
fn test_run_project_starts_dependencies() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Six;

    let dep1 = WorkerTestProject::One;
    let dep2 = WorkerTestProject::Two;

    let mut cmd = worker.run(project);
    cmd.assert().stdout("Hello from test!\n");

    assert_eq!(worker.pids(project).len(), 0);

    // Verify that the state file exists
    assert!(worker.state_file(dep1).is_some());
    assert_eq!(worker.pids(dep1).len(), 1);

    // Verify that the state file exists
    assert!(worker.state_file(dep2).is_some());
    assert_eq!(worker.pids(dep2).len(), 1);
}
