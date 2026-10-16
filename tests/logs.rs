use std::time::{Duration, Instant};

use common::WorkerTest;

mod common;

#[test]
fn test_logs_project_not_running() {
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "api"
            command = {}
            cwd = "/"
            "#,
            ctx.mock_command("api"),
        )
    });

    worker.logs(&["api"]).assert().failure();
}

#[test]
fn test_logs_success() {
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "api"
            command = {}
            cwd = "/"
            "#,
            ctx.mock_command("api"),
        )
    });

    worker.start(&["api"]).assert().success();

    // // Try multiple times since it may not output immediately
    let start = Instant::now();
    while let Err(error) = worker
        .logs(&["api"])
        .assert()
        .success()
        .try_stdout(predicates::str::contains("Hello from mock!"))
    {
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "Timed out waiting for logs: {error}"
        );
    }
}

#[test]
fn test_logs_command_success() {
    let worker = WorkerTest::default();

    worker
        .start(&["-n", "api", "-c", "echo 'Hello from api!' && sleep 5"])
        .assert()
        .success();

    // Try multiple times since it may not output immediately
    let start = Instant::now();
    while let Err(error) = worker
        .logs(&["api"])
        .assert()
        .success()
        .try_stdout(predicates::str::contains("Hello from api!"))
    {
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "Timed out waiting for logs: {error}"
        );
    }
}
