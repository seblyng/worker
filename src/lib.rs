pub mod config;
pub mod libc;
pub mod project;

use project::{Project, RunningProject};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionArg {
    Project(Project),
    Group(Vec<Project>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionArgRunning {
    Project(RunningProject),
    Group(Vec<RunningProject>),
}
