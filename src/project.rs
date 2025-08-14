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

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

use crate::{
    config::WorkerConfig,
    libc::{fork, has_processes_running, setsid, stop_pg, waitpid, Fork, Signal},
};

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
                config.store_state(sid, self)?;

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
                            .stdout(unsafe { Stdio::from_raw_fd(fd) })
                            .stderr(unsafe { Stdio::from_raw_fd(fd) })
                            .stdin(Stdio::null())
                            .exec();
                    }
                };
            }
        };

        Ok(())
    }

    pub fn start_dependencies(&self, config: &WorkerConfig) -> Result<(), anyhow::Error> {
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

    pub fn run(&self) -> Result<(), anyhow::Error> {
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
        Ok(config.running()?.iter().any(|it| it.name == self.name))
    }
}

impl RunningProject {
    pub fn stop(&self) -> Result<(), anyhow::Error> {
        let signal = self.stop_signal.as_ref().unwrap_or(&Signal::SIGINT);
        stop_pg(self.pid, signal).map_err(|e| anyhow!("Error trying to stop project: {e}"))
    }

    pub fn is_running(&self) -> bool {
        has_processes_running(self.pid)
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
        }
    }
}

pub trait WorkerProject {
    fn name(&self) -> &str;
}

impl Hash for Project {
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
        find_project(s, config)
    }
}

impl FromStr for RunningProject {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config = WorkerConfig::new()?;
        let project = find_project(s, config.clone()).unwrap_or_else(|_| Project {
            name: s.to_string(),
            cwd: std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            ..Default::default()
        });

        let pid = config
            .get_pid(&project)?
            .ok_or_else(|| anyhow!("{} is not running", project))?;

        if !has_processes_running(pid) {
            return Err(anyhow!("{} is not running", project));
        }

        Ok(RunningProject {
            name: project.name,
            command: project.command,
            cwd: project.cwd,
            display: project.display,
            stop_signal: project.stop_signal,
            envs: project.envs,
            group: project.group,
            pid,
            dependencies: project.dependencies,
        })
    }
}

fn find_project(name: &str, config: WorkerConfig) -> Result<Project, anyhow::Error> {
    let projects: Vec<String> = config.projects.iter().map(|p| p.name.clone()).collect();

    config
        .projects
        .into_iter()
        .find(|it| it.name == name)
        .with_context(|| format!("Valid projects are {:#?}", projects))
}
