use std::{
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, anyhow};
use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};

use crate::{
    ActionArg, ActionArgRunning, ProjectArg,
    project::{Project, RunningProject, WorkerProject},
};

const CONFIG_FILE: &str = ".worker.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub project: Vec<Project>,
}

impl FromStr for ActionArg {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config = WorkerConfig::new()?;

        if let Some(arg) = resolve(config.projects.iter().cloned(), s) {
            return Ok(arg);
        }

        Err(anyhow!("Unknown project or group: {s}"))
    }
}

impl FromStr for ActionArgRunning {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config = WorkerConfig::new()?;

        if let Some(arg) = resolve(config.running()?, s) {
            return Ok(arg);
        }

        if let Some(project) = config.projects.iter().find(|it| it.name == s) {
            println!("{} is not running", project);
        } else {
            println!("{} is not a project nor a running command", s);
        }

        Ok(ActionArgRunning::Group(vec![]))
    }
}

fn resolve<T: WorkerProject>(
    projects: impl IntoIterator<Item = T>,
    name: &str,
) -> Option<ProjectArg<T>> {
    let mut group = Vec::new();
    let mut project = None;

    for p in projects {
        if p.groups().iter().any(|g| g == name) {
            group.push(p);
        } else if project.is_none() && p.name() == name {
            project = Some(p);
        }
    }

    if group.is_empty() {
        project.map(ProjectArg::Project)
    } else {
        Some(ProjectArg::Group(group))
    }
}

#[derive(Clone)]
pub struct WorkerConfig {
    pub projects: Vec<Project>,
    state_dir: PathBuf,
    log_dir: PathBuf,
}

impl WorkerConfig {
    pub fn new() -> Result<Self, anyhow::Error> {
        let cwd = std::env::current_dir()?;
        let base_dir = cwd
            .ancestors()
            .find(|dir| dir.join(CONFIG_FILE).exists())
            .map(Path::to_path_buf)
            .context("Couldn't find config root")?;

        let config_string = std::fs::read_to_string(base_dir.join(CONFIG_FILE))?;

        let state_dir = base_dir.join(".worker/state");
        let log_dir = base_dir.join(".worker/log");

        std::fs::create_dir_all(&state_dir)?;
        std::fs::create_dir_all(&log_dir)?;

        // Deserialize the TOML string into the Config struct
        let config: Config = toml::from_str(&config_string)?;
        let config = Self {
            projects: config.project,
            state_dir: state_dir.clone(),
            log_dir,
        };

        for project in &config.projects {
            let path = state_dir.join(&project.name);
            if path.exists() {
                let str = std::fs::read_to_string(&path)?;
                let mut running_project = serde_json::from_str::<RunningProject>(&str)?;
                running_project.update(project);
                config.store_state(&running_project)?;
            }
        }

        Ok(config)
    }

    pub fn log_file<T: WorkerProject>(&self, project: &T) -> PathBuf {
        self.log_dir.join(project.name())
    }

    pub fn store_state(&self, project: &RunningProject) -> Result<(), anyhow::Error> {
        let state_file = self.state_dir.join(&project.name);

        let file = File::create(state_file).expect("Couldn't create state file");
        serde_json::to_writer(file, &project).expect("Couldn't write to state file");

        Ok(())
    }

    pub fn running(&self) -> Result<impl Iterator<Item = RunningProject>, anyhow::Error> {
        let projects = std::fs::read_dir(self.state_dir.as_path())?.filter_map(|entry| {
            let path = entry.ok()?.path();
            let str = std::fs::read_to_string(&path).ok()?;
            let project = serde_json::from_str::<RunningProject>(&str)
                .context("Couldn't parse project from state file")
                .ok()?;

            if !project.is_running() {
                return None;
            }

            Some(project)
        });

        Ok(projects)
    }

    pub fn partition_projects<T>(
        &self,
        projects: Vec<T>,
    ) -> Result<(Vec<RunningProject>, Vec<Project>), anyhow::Error>
    where
        T: WorkerProject + Into<Project>,
    {
        // Partition map to get project with pid set
        let mut running_projects = self.running()?;
        let (running, not_running): (Vec<_>, Vec<_>) = projects.into_iter().partition_map(|rp| {
            match running_projects.find(|p| p.name == rp.name()) {
                Some(p) => Either::Left(p.to_owned()),
                None => Either::Right(rp.into()),
            }
        });

        Ok((running, not_running))
    }
}

impl Drop for WorkerConfig {
    fn drop(&mut self) {
        let Ok(entries) = std::fs::read_dir(&self.state_dir) else {
            return;
        };

        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };

            let path = entry.path();

            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };

            let Ok(project) = serde_json::from_str::<RunningProject>(&contents) else {
                continue;
            };

            if !project.is_running() {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}
