pub mod config;
pub mod libc;
pub mod project;

use project::{Project, RunningProject};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectArg<T> {
    Project(T),
    Group(Vec<T>),
}

pub type ActionArg = ProjectArg<Project>;
pub type ActionArgRunning = ProjectArg<RunningProject>;
