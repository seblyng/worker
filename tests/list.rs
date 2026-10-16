use common::WorkerTest;
use predicates::prelude::predicate;

mod common;

#[test]
fn test_list_projects() {
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

    worker
        .list()
        .assert()
        .success()
        .stdout(predicate::str::contains("api"))
        .stdout(predicate::str::contains("database"))
        .stdout(predicate::str::contains("frontend"));
}
