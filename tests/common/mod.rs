#![allow(dead_code)]

use assert_cmd::{Command, cargo::cargo_bin};
use sysinfo::{Pid, System};
use tempfile::TempDir;
use uuid::Uuid;
use worker::{
    config::Config,
    project::{Project, RunningProject},
};

#[derive(Eq, PartialEq, Debug)]
pub enum PidError {
    FileNotFound,
    ParseError,
}

pub struct TestContext {
    instance_id: Uuid,
}

impl TestContext {
    /// A TOML command array, unique to this test and project. Not a shell command.
    pub fn mock_command(&self, project: &str) -> String {
        let args = vec![
            cargo_bin("mock").to_string_lossy().into_owned(),
            self.instance_id.to_string(),
            project.to_owned(),
        ];

        toml::Value::Array(args.into_iter().map(toml::Value::String).collect()).to_string()
    }
}

pub struct WorkerTest {
    dir: TempDir,
}

impl WorkerTest {
    pub fn with_config(config: impl FnOnce(&TestContext) -> String) -> Self {
        let dir = TempDir::new().unwrap();
        let context = TestContext {
            instance_id: Uuid::new_v4(),
        };

        std::fs::write(dir.path().join(".worker.toml"), config(&context)).unwrap();

        Self { dir }
    }

    pub fn default() -> Self {
        Self::with_config(|_| "project = []".into())
    }

    fn run_cmd(&self, command: &str, projects: Option<&[&str]>) -> Command {
        let mut cmd = Command::cargo_bin("worker").unwrap();
        cmd.current_dir(&self.dir).arg(command);

        if let Some(projects) = projects {
            cmd.args(projects);
        }

        cmd
    }

    pub fn start(&self, projects: &[&str]) -> Command {
        self.run_cmd("start", Some(projects))
    }

    pub fn logs(&self, project: &[&str]) -> Command {
        self.run_cmd("logs", Some(project))
    }

    pub fn run(&self, project: &[&str]) -> Command {
        self.run_cmd("run", Some(project))
    }

    pub fn restart(&self, projects: &[&str]) -> Command {
        self.run_cmd("restart", Some(projects))
    }

    pub fn stop(&self, projects: &[&str]) -> Command {
        self.run_cmd("stop", Some(projects))
    }

    pub fn list(&self) -> Command {
        self.run_cmd("list", None)
    }

    pub fn status(&self) -> Command {
        self.run_cmd("status", None)
    }

    pub fn update_config(&self, name: &str, update: impl FnOnce(&mut Project)) {
        let path = self.dir.path().join(".worker.toml");
        let contents = std::fs::read_to_string(&path).expect("Couldn't read config");
        let mut config: Config = toml::from_str(&contents).expect("Couldn't parse config");

        let project = config
            .project
            .iter_mut()
            .find(|project| project.name == name)
            .unwrap_or_else(|| panic!("Project {name:?} not found in config"));

        update(project);

        let contents = toml::to_string(&config).expect("Couldn't serialize config");
        std::fs::write(path, contents).expect("Couldn't write config");
    }

    pub fn pids(&self, name: &str) -> Result<Vec<Pid>, PidError> {
        let path = self.dir.path().join(".worker/state");
        let dir = path.read_dir().map_err(|_| PidError::FileNotFound)?;

        let mut sessions = Vec::new();
        for entry in dir {
            let entry = entry.unwrap().path();
            let project = std::fs::read_to_string(entry).map_err(|_| PidError::FileNotFound)?;
            let project = serde_json::from_str::<RunningProject>(&project)
                .map_err(|_| PidError::ParseError)?;

            if project.name != name {
                continue;
            }

            let command = project
                .command
                .into_iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>();

            sessions.push((Pid::from_u32(project.pid as u32), command));
        }

        if sessions.is_empty() {
            return Err(PidError::FileNotFound);
        }

        // HACK: Seems like I unfortunately need to sleep a bit here to make the tests less flaky
        // Seems like they need some time before they are recognized as processes.
        std::thread::sleep(std::time::Duration::from_millis(200));

        Ok(System::new_all()
            .processes()
            .values()
            .filter(|p| {
                sessions.iter().any(|(session_id, command)| {
                    p.session_id() == Some(*session_id) && p.cmd() == command.as_slice()
                })
            })
            .map(|p| p.pid())
            .collect())
    }
}
