use common::{PidError, WorkerTest};

mod common;

#[test]
fn test_run_project() {
    let worker = WorkerTest::with_config(|_| {
        r#"
        [[project]]
        name = "greet"
        command = ["sh", "-c", "echo 'Hello from test!'"]
        cwd = "/"
        "#
        .into()
    });

    worker
        .run(&["greet"])
        .assert()
        .success()
        .stdout("Hello from test!\n");

    assert_eq!(Err(PidError::FileNotFound), worker.pids("greet"));
}

#[test]
fn test_run_project_starts_dependencies() {
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "database"
            command = {}
            cwd = "/"

            [[project]]
            name = "cache"
            command = {}
            cwd = "/"

            [[project]]
            name = "greet"
            command = ["sh", "-c", "echo 'Hello from test!'"]
            cwd = "/"
            dependencies = ["database", "cache"]
            "#,
            ctx.mock_command("database"),
            ctx.mock_command("cache"),
        )
    });

    worker
        .run(&["greet"])
        .assert()
        .success()
        .stdout("Hello from test!\n");

    assert_eq!(Err(PidError::FileNotFound), worker.pids("greet"));
    assert_eq!(worker.pids("database").unwrap().len(), 1);
    assert_eq!(worker.pids("cache").unwrap().len(), 1);
}

#[test]
fn test_run_command_success() {
    let worker = WorkerTest::default();

    worker
        .run(&["-n", "api", "-c", "echo 'Hello from api!'"])
        .assert()
        .success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
}

#[test]
fn test_run_command_require_name() {
    let worker = WorkerTest::default();

    worker
        .run(&["-c", "echo 'Hello from api!'"])
        .assert()
        .failure();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
}
