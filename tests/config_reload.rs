use common::WorkerTest;
use predicates::prelude::{PredicateBooleanExt, predicate};

use crate::common::PidError;

mod common;

#[test]
fn test_reload_display() {
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "api"
            command = {}
            cwd = "/"
            display = "Old API"
            "#,
            ctx.mock_command("api"),
        )
    });

    worker.start(&["api"]).assert().success();

    worker
        .status()
        .assert()
        .success()
        .stdout(predicate::str::contains("Old API"));

    worker.update_config("api", |project| {
        project.display = Some("New API".to_string());
    });

    worker
        .status()
        .assert()
        .success()
        .stdout(predicate::str::contains("Old API").not())
        .stdout(predicate::str::contains("New API"));
}

#[test]
fn test_reload_group() {
    let worker = WorkerTest::with_config(|ctx| {
        format!(
            r#"
            [[project]]
            name = "api"
            command = {}
            cwd = "/"
            group = ["old"]
            "#,
            ctx.mock_command("api"),
        )
    });

    worker.start(&["old"]).assert().success();

    worker
        .stop(&["new"])
        .assert()
        .success()
        .stdout("new is not a project nor a running command\n");

    assert_eq!(worker.pids("api").unwrap().len(), 1);

    worker.update_config("api", |project| {
        project.group = Some(vec!["new".to_string()]);
    });

    worker.stop(&["new"]).assert().success();
    assert_eq!(Err(PidError::FileNotFound), worker.pids("api"));
}

#[test]
fn test_name_is_not_updated() {
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

    worker.update_config("api", |project| {
        project.name = "new_api".to_string();
    });

    worker
        .stop(&["new_api"])
        .assert()
        .success()
        .stdout("new_api is not running\n");

    assert_eq!(worker.pids("api").unwrap().len(), 1);
}

#[test]
fn test_reload_dependencies() {
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
            "#,
            ctx.mock_command("database"),
            ctx.mock_command("cache"),
            ctx.mock_command("api"),
        )
    });

    worker.start(&["api"]).assert().success();

    assert_eq!(worker.pids("api").unwrap().len(), 1);
    assert_eq!(Err(PidError::FileNotFound), worker.pids("database"));
    assert_eq!(Err(PidError::FileNotFound), worker.pids("cache"));

    worker.update_config("cache", |project| {
        project.dependencies = Some(vec!["database".to_string()]);
    });

    worker.start(&["cache"]).assert().success();

    assert_eq!(worker.pids("cache").unwrap().len(), 1);
    assert_eq!(worker.pids("database").unwrap().len(), 1);
}
