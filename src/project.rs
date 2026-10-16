use std::{
    collections::HashMap,
    fs::OpenOptions,
    hash::Hash,
    os::{
        fd::{FromRawFd, IntoRawFd},
        unix::process::CommandExt,
    },
    process::Stdio,
    str::FromStr,
};

use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};

use crate::{
    config::WorkerConfig,
    libc::{Fork, Signal, fork, has_processes_running, setsid, stop_pg, waitpid},
};

#[derive(Deserialize, Serialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct Lifecycle {
    pub stop: Option<Vec<String>>,
    pub logs: Option<Vec<String>>,
    pub status: Option<Vec<String>>,
}

/// Project deserialized from config file
#[derive(Deserialize, Serialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct Project {
    pub name: String,
    pub command: Vec<String>,
    pub cwd: String,
    pub display: Option<String>,
    pub stop_signal: Option<Signal>,
    pub envs: Option<HashMap<String, String>>,
    pub group: Option<Vec<String>>,
    pub dependencies: Option<Vec<String>>,
    pub lifecycle: Option<Lifecycle>,
}

/// Project with process id
#[derive(Deserialize, Serialize, Clone, Debug, Eq, PartialEq)]
pub struct RunningProject {
    pub name: String,
    pub command: Vec<String>,
    pub cwd: String,
    pub display: Option<String>,
    pub stop_signal: Option<Signal>,
    pub envs: Option<HashMap<String, String>>,
    pub group: Option<Vec<String>>,
    pub dependencies: Option<Vec<String>>,
    pub pid: i32,
    pub lifecycle: Option<Lifecycle>,
}

impl Project {
    pub fn from_cmd(name: String, cmd: String) -> Self {
        Project {
            name,
            command: vec!["sh".to_string(), "-c".to_string(), cmd],
            cwd: std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            display: None,
            stop_signal: None,
            envs: None,
            group: None,
            dependencies: None,
            lifecycle: None,
        }
    }

    pub fn start(&self, config: &WorkerConfig) -> Result<(), anyhow::Error> {
        self.start_dependencies(config)?;

        match fork().expect("Couldn't fork") {
            Fork::Parent(p) => {
                waitpid(p).unwrap();
            }
            Fork::Child => {
                let sid = setsid().expect("Couldn't setsid");
                let running = RunningProject {
                    name: self.name.clone(),
                    command: self.command.clone(),
                    cwd: self.cwd.clone(),
                    display: self.display.clone(),
                    stop_signal: self.stop_signal.clone(),
                    envs: self.envs.clone(),
                    group: self.group.clone(),
                    dependencies: self.dependencies.clone(),
                    pid: sid,
                    lifecycle: self.lifecycle.clone(),
                };

                config.store_state(&running)?;

                match fork().expect("Couldn't fork inner") {
                    Fork::Parent(_) => std::process::exit(0),
                    Fork::Child => {
                        // Create a raw filedescriptor to use to merge stdout and stderr
                        let fd = OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .create(true)
                            .open(config.log_file(self))?
                            .into_raw_fd();

                        let _ = std::process::Command::new(&self.command[0])
                            .args(&self.command[1..])
                            .envs(self.envs.clone().unwrap_or_default())
                            .current_dir(&self.cwd)
                            .stdin(Stdio::null())
                            .stdout(unsafe { Stdio::from_raw_fd(fd) })
                            .stderr(unsafe { Stdio::from_raw_fd(fd) })
                            .exec();
                    }
                };
            }
        };

        Ok(())
    }

    pub fn run(&self, config: &WorkerConfig) -> Result<(), anyhow::Error> {
        self.start_dependencies(config)?;

        let _ = std::process::Command::new(&self.command[0])
            .args(&self.command[1..])
            .envs(self.envs.clone().unwrap_or_default())
            .current_dir(&self.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
            .wait();
        Ok(())
    }

    pub fn is_running(&self, config: &WorkerConfig) -> Result<bool, anyhow::Error> {
        Ok(config.running()?.any(|it| it.name == self.name))
    }

    fn start_dependencies(&self, config: &WorkerConfig) -> Result<(), anyhow::Error> {
        if let Some(ref deps) = self.dependencies {
            for dep in deps {
                let project = Project::from_str(dep)?;
                if !project.is_running(config)? {
                    project.start(config)?;
                }
            }
        }
        Ok(())
    }
}

impl RunningProject {
    pub fn stop(&self) -> Result<(), anyhow::Error> {
        if let Some(ref lifecycle) = self.lifecycle
            && let Some(ref stop) = lifecycle.stop
        {
            let _ = std::process::Command::new(&stop[0])
                .args(&stop[1..])
                .envs(self.envs.clone().unwrap_or_default())
                .current_dir(&self.cwd)
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()?
                .wait();
            Ok(())
        } else {
            let signal = self.stop_signal.as_ref().unwrap_or(&Signal::SIGINT);
            stop_pg(self.pid, signal).map_err(|e| anyhow!("Error trying to stop project: {e}"))
        }
    }

    pub fn is_running(&self) -> bool {
        if let Some(ref lifecycle) = self.lifecycle
            && let Some(ref status) = lifecycle.status
        {
            std::process::Command::new(&status[0])
                .args(&status[1..])
                .envs(self.envs.clone().unwrap_or_default())
                .current_dir(&self.cwd)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        } else {
            has_processes_running(self.pid)
        }
    }

    pub fn update(&mut self, project: &Project) {
        self.command = project.command.clone();
        self.cwd = project.cwd.clone();
        self.display = project.display.clone();
        self.stop_signal = project.stop_signal.clone();
        self.envs = project.envs.clone();
        self.group = project.group.clone();
        self.dependencies = project.dependencies.clone();
        self.lifecycle = project.lifecycle.clone();
    }
}

impl From<RunningProject> for Project {
    fn from(value: RunningProject) -> Self {
        Self {
            name: value.name,
            command: value.command,
            cwd: value.cwd,
            display: value.display,
            stop_signal: value.stop_signal,
            envs: value.envs,
            group: value.group,
            dependencies: value.dependencies,
            lifecycle: value.lifecycle,
        }
    }
}

pub trait WorkerProject {
    fn name(&self) -> &str;
    fn groups(&self) -> &[String];
}

impl Hash for Project {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl Hash for RunningProject {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

macro_rules! impl_display {
    ($project:tt) => {
        impl std::fmt::Display for $project {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                if let Some(ref display) = self.display {
                    write!(f, "{} ({})", display, self.name)
                } else {
                    write!(f, "{}", self.name)
                }
            }
        }
    };
}

macro_rules! impl_worker_project {
    ($project:tt) => {
        impl WorkerProject for $project {
            fn name(&self) -> &str {
                &self.name
            }

            fn groups(&self) -> &[String] {
                self.group.as_deref().unwrap_or_default()
            }
        }
    };
}

impl_display!(Project);
impl_display!(RunningProject);

impl_worker_project!(Project);
impl_worker_project!(RunningProject);

impl FromStr for Project {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config = WorkerConfig::new()?;
        let projects: Vec<String> = config.projects.iter().map(|p| p.name.clone()).collect();

        config
            .projects
            .clone()
            .into_iter()
            .find(|it| it.name == s)
            .with_context(|| format!("Valid projects are {:#?}", projects))
    }
}

impl FromStr for RunningProject {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config = WorkerConfig::new()?;
        let project = config
            .running()?
            .find(|it| it.name == s)
            .ok_or_else(|| anyhow!("Project {} is not running or does not exist", s))?;

        Ok(project)
    }
}
