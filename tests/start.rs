use common::{PidError, WorkerTest};

mod common;

#[test]
fn test_start_project_already_running() {
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
    worker
        .start(&["api"])
        .assert()
        .stderr("api is already running\n");

    assert_eq!(worker.pids("api").unwrap().len(), 1);
}

#[test]
fn test_start_unknown_project() {
    let worker = WorkerTest::default();

    worker.start(&["unknown"]).assert().failure();
}

#[test]
fn test_start_success() {
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

    assert_eq!(worker.pids("api").unwrap().len(), 1);
}

#[test]
fn test_start_multiple_success() {
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "api"
            command = {}
            cwd = "/"

            [[project]]
            name = "database"
            command = {}
            cwd = "/"
            "#,
            ctx.mock_command("api"),
            ctx.mock_command("database"),
        )
    });

    worker.start(&["api", "database"]).assert().success();

    assert_eq!(worker.pids("api").unwrap().len(), 1);
    assert_eq!(worker.pids("database").unwrap().len(), 1);
}

#[test]
fn test_start_multiple_one_already_running() {
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "api"
            command = {}
            cwd = "/"

            [[project]]
            name = "database"
            command = {}
            cwd = "/"
            "#,
            ctx.mock_command("api"),
            ctx.mock_command("database"),
        )
    });

    worker.start(&["api"]).assert().success();

    let pid = worker.pids("api").unwrap()[0];

    worker.start(&["api", "database"]).assert().success();

    assert_eq!(worker.pids("api").unwrap(), vec![pid]);
    assert_eq!(worker.pids("database").unwrap().len(), 1);
}

#[test]
fn test_start_group_success() {
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "api"
            command = {}
            cwd = "/"
            group = ["backend"]

            [[project]]
            name = "database"
            command = {}
            cwd = "/"
            group = ["backend"]
            "#,
            ctx.mock_command("api"),
            ctx.mock_command("database"),
        )
    });

    worker.start(&["backend"]).assert().success();

    assert_eq!(worker.pids("api").unwrap().len(), 1);
    assert_eq!(worker.pids("database").unwrap().len(), 1);
}

#[test]
fn test_start_group_and_project_success() {
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "api"
            command = {}
            cwd = "/"
            group = ["backend"]

            [[project]]
            name = "database"
            command = {}
            cwd = "/"
            group = ["backend"]

            [[project]]
            name = "frontend"
            command = {}
            cwd = "/"
            "#,
            ctx.mock_command("api"),
            ctx.mock_command("database"),
            ctx.mock_command("frontend"),
        )
    });

    worker.start(&["backend", "frontend"]).assert().success();

    assert_eq!(worker.pids("api").unwrap().len(), 1);
    assert_eq!(worker.pids("database").unwrap().len(), 1);
    assert_eq!(worker.pids("frontend").unwrap().len(), 1);
}

#[test]
fn test_start_multiple_groups() {
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "api"
            command = {}
            cwd = "/"
            group = ["backend", "web"]

            [[project]]
            name = "database"
            command = {}
            cwd = "/"
            group = ["backend"]

            [[project]]
            name = "frontend"
            command = {}
            cwd = "/"
            group = ["web"]
            "#,
            ctx.mock_command("api"),
            ctx.mock_command("database"),
            ctx.mock_command("frontend"),
        )
    });

    worker.start(&["backend", "web"]).assert().success();

    assert_eq!(worker.pids("api").unwrap().len(), 1);
    assert_eq!(worker.pids("database").unwrap().len(), 1);
    assert_eq!(worker.pids("frontend").unwrap().len(), 1);
}

#[test]
fn test_start_starts_dependencies() {
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
            name = "api"
            command = {}
            cwd = "/"
            dependencies = ["database", "cache"]
            "#,
            ctx.mock_command("database"),
            ctx.mock_command("cache"),
            ctx.mock_command("api"),
        )
    });

    worker.start(&["api"]).assert().success();

    assert_eq!(worker.pids("api").unwrap().len(), 1);
    assert_eq!(worker.pids("database").unwrap().len(), 1);
    assert_eq!(worker.pids("cache").unwrap().len(), 1);
}

#[test]
fn test_start_command_success() {
    let worker = WorkerTest::default();

    worker
        .start(&["-n", "api", "-c", "echo 'Hello from api!' && sleep 5"])
        .assert()
        .success();

    assert_eq!(worker.pids("api").unwrap().len(), 1);
}

#[test]
fn test_start_command_require_name() {
    let worker = WorkerTest::default();

    worker
        .start(&["-c", "echo 'Hello from api!' && sleep 5"])
        .assert()
        .failure();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
}
