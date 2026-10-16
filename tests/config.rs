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
        .failure()
        .stderr(predicate::str::contains("invalid value 'new'"));

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

#[test]
fn test_help_without_config() {
    let dir = tempfile::TempDir::new().unwrap();

    for args in [vec!["--help"], vec!["start", "--help"]] {
        assert_cmd::Command::cargo_bin("worker")
            .unwrap()
            .current_dir(dir.path())
            .args(args)
            .assert()
            .success()
            .stdout(predicates::str::contains("Usage:"));
    }
}

#[test]
fn test_unknown_name_lists_projects_and_groups() {
    let worker = WorkerTest::with_config(|_| {
        r#"
        [[project]]
        name = "api"
        command = ["true"]
        cwd = "/"
        group = ["backend"]
        "#
        .into()
    });

    for mut command in [
        worker.start(&["unknown"]),
        worker.stop(&["unknown"]),
        worker.restart(&["unknown"]),
    ] {
        command
            .assert()
            .failure()
            .stderr(predicates::str::contains("invalid value 'unknown'"))
            .stderr(predicates::str::contains("[possible values: api, backend]"));
    }
}

#[test]
fn test_run_invalid_name_lists_only_projects() {
    let worker = WorkerTest::with_config(|_| {
        r#"
        [[project]]
        name = "api"
        command = ["true"]
        cwd = "/"
        group = ["backend"]
        "#
        .into()
    });

    for name in ["unknown", "backend"] {
        worker
            .run(&[name])
            .assert()
            .failure()
            .stderr(predicates::str::contains(format!("invalid value '{name}'")))
            .stderr(predicates::str::contains("[possible values: api]"));
    }
}
