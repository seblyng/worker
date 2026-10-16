use common::WorkerTest;
use predicates::{boolean::PredicateBooleanExt, prelude::predicate};

mod common;

#[test]
fn test_status_none_running() {
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

    worker.status().assert().success().stdout("");
}

#[test]
fn test_status_one_running() {
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

    worker
        .status()
        .assert()
        .success()
        .stdout(predicate::str::contains("api"))
        .stdout(predicate::str::contains("database").not());
}

#[test]
fn test_status_multiple_running() {
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

    worker.start(&["api", "database"]).assert().success();

    worker
        .status()
        .assert()
        .success()
        .stdout(predicate::str::contains("api"))
        .stdout(predicate::str::contains("database"))
        .stdout(predicate::str::contains("frontend").not());
}

#[test]
fn test_status_uses_lifecycle_for_status() {
    let worker = WorkerTest::with_config(|_| {
        r#"
            [[project]]
            name = "api"
            command = ["false"]
            cwd = "/"

            [project.lifecycle]
            status = ["true"]
            "#
        .to_string()
    });

    worker.start(&["api"]).assert().success();

    worker
        .status()
        .assert()
        .success()
        .stdout("api is running\n");
}

#[test]
fn test_failed_lifecycle_status_removes_state() {
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "api"
            command = {}
            cwd = "/"

            [project.lifecycle]
            status = ["false"]
            "#,
            ctx.mock_command("api"),
        )
    });

    worker.start(&["api"]).assert().success();
    worker.status().assert().success().stdout("");
}
