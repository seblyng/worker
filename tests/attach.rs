use std::time::Duration;

use common::{WorkerTestConfig, WorkerTestProject};
use uuid::Uuid;

mod common;

#[test]
fn test_attach_project_not_running() {
    let worker = WorkerTestConfig::new();

    let project_name = worker.project_name(&WorkerTestProject::One);

    let mut cmd = worker.attach(&[&project_name]);
    cmd.assert().failure();
}

#[test]
fn test_attach_shows_previous_output() {
    let worker = WorkerTestConfig::new();

    let project_name = worker.project_name(&WorkerTestProject::One);

    let mut cmd = worker.start(&[&project_name]);
    cmd.assert().success();

    // Give the process time to produce output
    std::thread::sleep(Duration::from_millis(500));

    // Attach — server sends screen state on connect, then timeout
    let mut cmd = worker.attach(&[&project_name]);
    cmd.timeout(Duration::from_secs(2));
    let output = cmd.output().expect("Failed to run attach");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Hello from mock!"),
        "Expected previous output in attach stdout, got: {}",
        stdout
    );
}

#[test]
fn test_attach_detach_process_still_running() {
    let worker = WorkerTestConfig::new();

    let project_name = worker.project_name(&WorkerTestProject::One);

    let mut cmd = worker.start(&[&project_name]);
    cmd.assert().success();

    // Attach and immediately detach with Ctrl+D
    let mut cmd = worker.attach(&[&project_name]);
    let output = cmd
        .write_stdin([0x04])
        .output()
        .expect("Failed to run attach");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Detached"),
        "Expected detach message in stdout, got: {}",
        stdout
    );

    // Verify the process is still running after detach
    assert_eq!(worker.pids(&project_name).unwrap().len(), 1);
}

#[test]
fn test_attach_command_success() {
    let worker = WorkerTestConfig::new();

    let uuid = Uuid::new_v4();
    let echo_cmd = format!("echo 'attach-test-{}' && sleep 5", uuid);

    let mut cmd = worker.start(&["-n", &uuid.to_string(), "-c", &echo_cmd]);
    cmd.assert().success();

    // Give the process time to produce output
    std::thread::sleep(Duration::from_millis(500));

    // Attach — server sends screen state, then timeout
    let mut cmd = worker.attach(&[&uuid.to_string()]);
    cmd.timeout(Duration::from_secs(2));
    let output = cmd.output().expect("Failed to run attach");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains(&format!("attach-test-{}", uuid)),
        "Expected command output in attach stdout, got: {}",
        stdout
    );
}

#[test]
fn test_attach_process_exits_while_attached() {
    let worker = WorkerTestConfig::new();

    let uuid = Uuid::new_v4();
    let echo_cmd = format!("echo 'short-lived-{}' && sleep 1", uuid);

    let mut cmd = worker.start(&["-n", &uuid.to_string(), "-c", &echo_cmd]);
    cmd.assert().success();

    // Attach with Ctrl+D queued — process will exit after ~1s
    let mut cmd = worker.attach(&[&uuid.to_string()]);
    cmd.timeout(Duration::from_secs(5));
    let output = cmd
        .write_stdin([0x04])
        .output()
        .expect("Failed to run attach");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Process exited") || stdout.contains("Detached"),
        "Expected exit or detach message, got: {}",
        stdout
    );
}

#[test]
fn test_attach_sends_terminal_size() {
    let worker = WorkerTestConfig::new();

    let uuid = Uuid::new_v4();
    // Process that prints its terminal size repeatedly (bounded to avoid leaking)
    let cmd_str = format!(
        "echo 'ready-{}'; for i in $(seq 1 30); do stty size; sleep 0.2; done",
        uuid
    );

    let mut cmd = worker.start(&["-n", &uuid.to_string(), "-c", &cmd_str]);
    cmd.assert().success();

    // Give the process time to start
    std::thread::sleep(Duration::from_millis(500));

    // Attach — sends terminal size (24x80 in test env) in the handshake.
    // Then detach.
    let mut cmd = worker.attach(&[&uuid.to_string()]);
    cmd.write_stdin([0x04])
        .output()
        .expect("Failed to run attach");

    // Wait a bit then attach again — the size output should be in the screen state
    std::thread::sleep(Duration::from_millis(500));

    let mut cmd = worker.attach(&[&uuid.to_string()]);
    cmd.timeout(Duration::from_secs(2));
    let output = cmd.output().expect("Failed to run attach");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("24 80"),
        "Expected terminal size '24 80' in output, got: {}",
        stdout
    );

    worker.stop(&[&uuid.to_string()]).assert().success();
}

#[test]
fn test_attach_multiple_clients() {
    let worker = WorkerTestConfig::new();

    let project_name = worker.project_name(&WorkerTestProject::One);

    let mut cmd = worker.start(&[&project_name]);
    cmd.assert().success();

    // Give the process time to produce output
    std::thread::sleep(Duration::from_millis(500));

    // Attach first client — let it run briefly
    let mut cmd1 = worker.attach(&[&project_name]);
    cmd1.timeout(Duration::from_secs(2));
    let output1 = cmd1.output().expect("Failed to run first attach");
    let stdout1 = String::from_utf8_lossy(&output1.stdout);

    // Attach second client — let it run briefly
    let mut cmd2 = worker.attach(&[&project_name]);
    cmd2.timeout(Duration::from_secs(2));
    let output2 = cmd2.output().expect("Failed to run second attach");
    let stdout2 = String::from_utf8_lossy(&output2.stdout);

    // Both clients should have received output
    assert!(
        stdout1.contains("Hello from mock!"),
        "First client should have received output, got: {}",
        stdout1
    );
    assert!(
        stdout2.contains("Hello from mock!"),
        "Second client should have received output, got: {}",
        stdout2
    );
}
