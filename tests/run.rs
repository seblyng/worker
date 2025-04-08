use common::{WorkerTestConfig, WorkerTestProject};

mod common;

#[test]
fn test_run_project() {
    let worker = WorkerTestConfig::new();
    let project = WorkerTestProject::Four;

    let mut cmd = worker.run(project);
    cmd.assert().success();
    cmd.assert().stdout("Hello from test!\n");

    assert_eq!(worker.pids(project).len(), 0);
}
