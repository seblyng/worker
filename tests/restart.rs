use common::{PidError, WorkerTest};

mod common;

#[test]
fn test_restart_unknown_project() {
    let worker = WorkerTest::default();

    worker
        .restart(&["unknown"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("invalid value 'unknown'"));
}

#[test]
fn test_restart_success() {
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

    let pid = worker.pids("api").unwrap()[0];

    worker.restart(&["api"]).assert().success();

    assert_ne!(pid, worker.pids("api").unwrap()[0]);
}

#[test]
fn test_restart_multiple_success() {
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

    let api_pid = worker.pids("api").unwrap()[0];
    let database_pid = worker.pids("database").unwrap()[0];

    worker.restart(&["api", "database"]).assert().success();

    assert_ne!(api_pid, worker.pids("api").unwrap()[0]);
    assert_ne!(database_pid, worker.pids("database").unwrap()[0]);
}

#[test]
fn test_restart_multiple_only_one_running() {
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

    worker.restart(&["api", "database"]).assert().success();

    assert_ne!(pid, worker.pids("api").unwrap()[0]);
    assert_eq!(Err(PidError::FileNotFound), worker.pids("database"));
}

#[test]
fn test_restart_group_success() {
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

    let api_pid = worker.pids("api").unwrap()[0];
    let database_pid = worker.pids("database").unwrap()[0];

    worker.restart(&["backend"]).assert().success();

    assert_ne!(api_pid, worker.pids("api").unwrap()[0]);
    assert_ne!(database_pid, worker.pids("database").unwrap()[0]);
}

#[test]
fn test_restart_multiple_group_success() {
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

    let api_pid = worker.pids("api").unwrap()[0];
    let database_pid = worker.pids("database").unwrap()[0];
    let frontend_pid = worker.pids("frontend").unwrap()[0];

    worker.restart(&["backend", "web"]).assert().success();

    assert_ne!(api_pid, worker.pids("api").unwrap()[0]);
    assert_ne!(database_pid, worker.pids("database").unwrap()[0]);
    assert_ne!(frontend_pid, worker.pids("frontend").unwrap()[0]);
}

#[test]
fn test_restart_group_one_project_running() {
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

    worker.start(&["api"]).assert().success();

    let pid = worker.pids("api").unwrap()[0];

    worker.restart(&["backend"]).assert().success();

    assert_ne!(pid, worker.pids("api").unwrap()[0]);
    assert_eq!(Err(PidError::FileNotFound), worker.pids("database"));
}

#[test]
fn test_restart_not_restarting_dependencies() {
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

    worker.start(&["api", "api"]).assert().success();

    let api_pid = worker.pids("api").unwrap()[0];
    let database_pid = worker.pids("database").unwrap()[0];
    let cache_pid = worker.pids("cache").unwrap()[0];

    worker.restart(&["api", "api"]).assert().success();

    assert_ne!(api_pid, worker.pids("api").unwrap()[0]);
    assert_eq!(database_pid, worker.pids("database").unwrap()[0]);
    assert_eq!(cache_pid, worker.pids("cache").unwrap()[0]);
}

#[test]
fn test_restart_command_success() {
    let worker = WorkerTest::default();

    worker
        .start(&["-n", "api", "-c", "echo 'Hello from api!' && sleep 5"])
        .assert()
        .success();

    let pid = worker.pids("api").unwrap()[0];

    worker.restart(&["api"]).assert().success();

    assert_ne!(pid, worker.pids("api").unwrap()[0]);
}

#[test]
fn test_restart_uses_lifecycle() {
    let marker = uuid::Uuid::new_v4();
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "api"
            command = {}
            cwd = "/"

            [project.lifecycle]
            stop = ["sh", "-c", "pkill -f '[w]orker-restart-{marker}' && echo 'Stopped by lifecycle!'"]
            "#,
            ctx.mock_command(&format!("worker-restart-{marker}")),
        )
    });

    worker.start(&["api"]).assert().success();

    let pid = worker.pids("api").unwrap()[0];

    worker
        .restart(&["api"])
        .assert()
        .success()
        .stdout("Stopped by lifecycle!\n");

    let pids = worker.pids("api").unwrap();

    assert_eq!(pids.len(), 1);
    assert_ne!(pid, pids[0]);
}
