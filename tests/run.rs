use common::{WorkerTestConfig, WorkerTestProject};

mod common;

#[test]
fn test_run_project() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Four;

    let project_name = worker.project_name(&project);

    let mut cmd = worker.run(&[&project_name]);
    cmd.assert().success().stdout("Hello from test!\n");

    assert_eq!(worker.pids(project).len(), 0);
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

    assert_eq!(worker.pids(project).len(), 0);

    // Verify that the state file exists
    assert!(worker.state_file(&dep1_name).is_some());
    assert_eq!(worker.pids(dep1).len(), 1);

    // Verify that the state file exists
    assert!(worker.state_file(&dep2_name).is_some());
    assert_eq!(worker.pids(dep2).len(), 1);
}

// #[test]
// fn test_run_command() {
//     let worker = WorkerTestConfig::new();
//
//     let mut cmd = worker.run(&["-n", "foobar", "-c", "echo 'Hello from command!'"]);
//     cmd.assert().success().stdout("Hello from command!\n");
//
//     // assert_eq!(worker.pids(project).len(), 0);
// }
