use std::{fs::File, path::PathBuf, str::FromStr};

use anyhow::{anyhow, Context};
use itertools::{Either, Itertools};
use serde::Deserialize;

use crate::{
    libc::has_processes_running,
    project::{Project, RunningProject, WorkerProject},
    ActionArg,
};

const CONFIG_FILE: &str = ".worker.toml";

#[derive(Deserialize, Debug)]
pub struct Config {
    pub project: Vec<Project>,
}

impl FromStr for ActionArg {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config = WorkerConfig::new()?;

        let projects_in_group: Vec<_> = config
            .projects
            .clone()
            .into_iter()
            .filter(|it| {
                it.group
                    .as_ref()
                    .is_some_and(|group| group.contains(&s.to_string()))
            })
            .collect();

        if !projects_in_group.is_empty() {
            Ok(ActionArg::Group(projects_in_group))
        } else if let Some(project) = config.projects.iter().find(|it| it.name == s) {
            Ok(ActionArg::Project(project.clone()))
        } else {
            let project_names: Vec<String> =
                config.projects.iter().map(|p| p.name.clone()).collect();

            let group_names: Vec<_> = config
                .projects
                .iter()
                .filter_map(|it| it.group.clone())
                .flatten()
                .unique()
                .collect();

            Err(anyhow!(
                "\nValid projects are {:#?}\n\nValid groups are {:#?}",
                project_names,
                group_names
            ))
        }
    }
}

pub struct WorkerConfig {
    pub projects: Vec<Project>,
    state_dir: PathBuf,
    log_dir: PathBuf,
}

impl WorkerConfig {
    pub fn new() -> Result<Self, anyhow::Error> {
        let base_dir = find_config_dir()?.context("Couldn't find config dir")?;
        let config_string = std::fs::read_to_string(base_dir.join(CONFIG_FILE))?;

        let state_dir = base_dir.join(".worker/state");
        let log_dir = base_dir.join(".worker/log");

        std::fs::create_dir_all(&state_dir)?;
        std::fs::create_dir_all(&log_dir)?;

        // Deserialize the TOML string into the Config struct
        let config: Config = toml::from_str(&config_string)?;

        Ok(Self {
            projects: config.project,
            state_dir,
            log_dir,
        })
    }

    pub fn log_file(&self, project: &Project) -> PathBuf {
        self.log_dir.join(&project.name)
    }

    pub fn store_state(&self, pid: i32, project: &Project) -> Result<(), anyhow::Error> {
        let filename = format!("{}-{}", project.name, pid);
        let state_file = self.state_dir.join(filename);

        let file = File::create(state_file).expect("Couldn't create state file");
        serde_json::to_writer(file, &project).expect("Couldn't write to state file");

        Ok(())
    }

    // Try to get vec of running projects. Try to remove the state file if the process is not running
    pub fn running(&self) -> Result<Vec<RunningProject>, anyhow::Error> {
        let projects = std::fs::read_dir(self.state_dir.as_path())?
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let project = RunningProject::from_str(path.file_name()?.to_str()?).ok()?;
                if has_processes_running(project.pid) {
                    Some(project)
                } else {
                    let _ = std::fs::remove_file(&path);
                    None
                }
            })
            .collect::<Vec<_>>();

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
        let running_projects = self.running()?;
        let (running, not_running): (Vec<_>, Vec<_>) = projects.into_iter().partition_map(|rp| {
            match running_projects.iter().find(|p| p.name == rp.name()) {
                Some(p) => Either::Left(p.to_owned()),
                None => Either::Right(rp.into()),
            }
        });

        Ok((running, not_running))
    }
}

// Scan root directories until we hopefully find the config file
fn find_config_dir() -> Result<Option<PathBuf>, anyhow::Error> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join(CONFIG_FILE).exists() {
            return Ok(Some(dir));
        }
        if let Some(parent) = dir.parent() {
            dir = parent.to_path_buf();
        } else {
            return Ok(None);
        }
    }
}
