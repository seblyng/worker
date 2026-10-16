use common::{PidError, WorkerTest};

mod common;

#[test]
fn test_stop_unknown_project() {
    let worker = WorkerTest::default();

    worker
        .stop(&["unknown"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("invalid value 'unknown'"));
}

#[test]
fn test_stop_command_not_running() {
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

    worker
        .stop(&["api"])
        .assert()
        .success()
        .stdout("api is not running\n");
}

#[test]
fn test_stop_success() {
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
    worker.stop(&["api"]).assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
}

#[test]
fn test_stop_success_one_still_running() {
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
    worker.stop(&["api"]).assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
    assert_eq!(worker.pids("database").unwrap().len(), 1);
}

#[test]
fn test_stop_multiple_success() {
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
    worker.stop(&["api", "database"]).assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
    assert_eq!(Err(PidError::FileNotFound), worker.pids("database"));
}

#[test]
fn test_stop_multiple_one_already_stopped() {
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
    worker.stop(&["api"]).assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));

    worker.stop(&["api", "database"]).assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
    assert_eq!(Err(PidError::FileNotFound), worker.pids("database"));
}

#[test]
fn test_stop_group_success() {
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
    worker.stop(&["backend"]).assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
    assert_eq!(Err(PidError::FileNotFound), worker.pids("database"));
}

#[test]
fn test_stop_multiple_groups_success() {
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
    worker.stop(&["backend", "web"]).assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
    assert_eq!(Err(PidError::FileNotFound), worker.pids("database"));
    assert_eq!(Err(PidError::FileNotFound), worker.pids("frontend"));
}

#[test]
fn test_stop_groups_and_project_success() {
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
    worker.stop(&["backend", "frontend"]).assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
    assert_eq!(Err(PidError::FileNotFound), worker.pids("database"));
    assert_eq!(Err(PidError::FileNotFound), worker.pids("frontend"));
}

#[test]
fn test_stop_not_stop_dependencies() {
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
    worker.stop(&["api"]).assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
    assert_eq!(worker.pids("database").unwrap().len(), 1);
    assert_eq!(worker.pids("cache").unwrap().len(), 1);
}

#[test]
fn test_stop_command_success() {
    let worker = WorkerTest::default();

    worker
        .start(&["-n", "api", "-c", "echo 'Hello from api!' && sleep 5"])
        .assert()
        .success();

    worker.stop(&["api"]).assert().success();

    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
}

#[test]
fn test_stop_uses_lifecycle() {
    let marker = uuid::Uuid::new_v4();
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "api"
            command = {}
            cwd = "/"

            [project.lifecycle]
            stop = ["sh", "-c", "pkill -f '[w]orker-stop-{marker}' && echo 'Stopped by lifecycle!'"]
            "#,
            ctx.mock_command(&format!("worker-stop-{marker}")),
        )
    });

    worker.start(&["api"]).assert().success();

    worker
        .stop(&["api"])
        .assert()
        .success()
        .stdout("Stopped by lifecycle!\n");

    worker.status().assert().success().stdout("");
    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
}
