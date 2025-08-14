use std::time::{Duration, Instant};

use clap::{command, ArgGroup, Parser};
use config::WorkerConfig;
use itertools::Itertools;
use project::Project;

use crate::project::RunningProject;

pub mod config;
pub mod libc;
pub mod project;

fn start(config: &WorkerConfig, projects: Vec<Project>) -> Result<(), anyhow::Error> {
    let (running, not_running) = config.partition_projects(projects)?;

    for project in running {
        eprintln!("{} is already running", project);
    }

    for project in not_running {
        project.start(config)?;
    }

    Ok(())
}

fn stop(config: &WorkerConfig, projects: Vec<RunningProject>) -> Result<(), anyhow::Error> {
    for project in projects.iter() {
        project.stop()?;
    }

    let timeout = Duration::new(5, 0);
    let start = Instant::now();

    while Instant::now().duration_since(start) < timeout {
        let (still_running, _) = config.partition_projects(projects.clone())?;
        if still_running.is_empty() {
            return Ok(());
        }
    }

    let (still_running, _) = config.partition_projects(projects)?;
    for p in still_running {
        eprintln!("Was not able to stop {}", p);
    }

    Ok(())
}

fn restart(config: &WorkerConfig, projects: Vec<RunningProject>) -> Result<(), anyhow::Error> {
    stop(config, projects.clone())?;
    start(config, projects.into_iter().map(|p| p.into()).collect())?;

    Ok(())
}

fn run(config: &WorkerConfig, project: Project) -> Result<(), anyhow::Error> {
    project.start_dependencies(config)?;

    project.run()?;

    Ok(())
}

fn status(config: &WorkerConfig, args: StatusArgs) -> Result<(), anyhow::Error> {
    for project in config.running()? {
        if args.quiet {
            println!("{}", project.name);
        } else {
            println!("{} is running", project);
        }
    }

    Ok(())
}

fn list(config: &WorkerConfig, args: ListArgs) -> Result<(), anyhow::Error> {
    for p in config.projects.iter() {
        if args.quiet {
            println!("{}", p.name)
        } else {
            println!("{}", p)
        }
    }

    Ok(())
}

fn logs(config: &WorkerConfig, args: LogsArgs) -> Result<(), anyhow::Error> {
    let mut cmd = std::process::Command::new("tail");

    if args.follow {
        cmd.arg("-f");
    }

    let mut child = cmd
        .args(["-n", &args.number.to_string()])
        .arg(config.log_file(&args.project))
        .spawn()?;

    if args.follow {
        while args.project.is_running() {
            std::thread::sleep(Duration::from_secs(2));
        }
        child.kill()?;
    } else {
        child.wait()?;
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ActionArg {
    Project(Project),
    Group(Vec<Project>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ActionArgRunning {
    Project(RunningProject),
    Group(Vec<RunningProject>),
}

#[derive(Debug, Parser)]
struct ActionArgs {
    projects: Vec<ActionArg>,
}

#[derive(Debug, Parser)]
struct ActionArgsRunning {
    projects: Vec<ActionArgRunning>,
}

#[derive(Debug, Parser)]
#[command(group(
    ArgGroup::new("mode")
        .required(true)
        .multiple(false)
        .args(["projects", "cmd"])
))]
struct StartArgs {
    #[arg(value_name = "PROJECTS", id = "projects", conflicts_with_all = ["cmd", "name"])]
    projects: Option<Vec<ActionArg>>,

    /// One-off cmd (mutually exclusive with PROJECTS)
    #[arg(
        short = 'c',
        long = "cmd",
        value_name = "CMD",
        id = "cmd",
        requires = "name",
        conflicts_with = "projects"
    )]
    cmd: Option<String>,

    /// Optional name for one-off, only valid with --cmd
    #[arg(
        short = 'n',
        long = "name",
        requires = "cmd",
        conflicts_with = "projects"
    )]
    name: Option<String>,
}

#[derive(Debug, Parser)]
#[command(group(
    ArgGroup::new("mode")
        .required(true)
        .multiple(false)
        .args(["project", "cmd"])
))]
struct RunArgs {
    #[arg(value_name = "PROJECT", id = "project", conflicts_with_all = ["cmd", "name"])]
    project: Option<Project>,

    /// One-off cmd (mutually exclusive with PROJECT)
    #[arg(
        short = 'c',
        long = "cmd",
        value_name = "CMD",
        id = "cmd",
        requires = "name",
        conflicts_with = "project"
    )]
    cmd: Option<String>,

    /// Optional name for one-off, only valid with --cmd
    #[arg(
        short = 'n',
        long = "name",
        requires = "cmd",
        conflicts_with = "project"
    )]
    name: Option<String>,
}

#[derive(Debug, Parser)]
struct LogsArgs {
    project: RunningProject,
    #[arg(short, long)]
    follow: bool,

    #[arg(short, long = "lines", default_value = "50")]
    number: i32,
}

#[derive(Debug, Parser)]
struct StatusArgs {
    #[arg(short, long, help = "Only print name of the project")]
    quiet: bool,
}

#[derive(Debug, Parser)]
struct ListArgs {
    #[arg(short, long, help = "Only print name of the project")]
    quiet: bool,
}

#[derive(Parser, Debug)]
enum SubCommands {
    /// Start the specified project(s). E.g. `worker start foo bar`
    Start(StartArgs),
    /// Stop the specified project(s). E.g. `worker stop foo bar`
    Stop(ActionArgsRunning),
    /// Restart the specified project(s). E.g. `worker restart foo bar` (Same as running stop and then start)
    Restart(ActionArgsRunning),
    /// Runs the project in the foreground
    Run(RunArgs),
    /// Print out a status of which projects is running
    Status(StatusArgs),
    /// Print out a list of available projects to run
    List(ListArgs),
    /// Print out logs for the specified project.
    Logs(LogsArgs),
}

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    subcommand: SubCommands,
}

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let config = WorkerConfig::new()?;

    let unique = |projects: Vec<ActionArg>| {
        projects
            .into_iter()
            .flat_map(|it| match it {
                ActionArg::Project(project) => vec![project],
                ActionArg::Group(vec) => vec,
            })
            .unique()
            .collect()
    };

    match cli.subcommand {
        SubCommands::Start(args) => {
            let projects = match (args.projects, args.name, args.cmd) {
                (Some(projects), None, None) => unique(projects),
                (None, Some(name), Some(command)) => vec![Project::from_cmd(name, command)],
                _ => unreachable!("Only one of project or command should be specified"),
            };

            start(&config, projects)?
        }
        SubCommands::Stop(args) => stop(
            &config,
            args.projects
                .into_iter()
                .flat_map(|it| match it {
                    ActionArgRunning::Project(project) => vec![project],
                    ActionArgRunning::Group(vec) => vec,
                })
                .unique()
                .collect(),
        )?,

        SubCommands::Restart(args) => restart(
            &config,
            args.projects
                .into_iter()
                .flat_map(|it| match it {
                    ActionArgRunning::Project(project) => vec![project],
                    ActionArgRunning::Group(vec) => vec,
                })
                .unique()
                .collect(),
        )?,
        SubCommands::Run(args) => {
            let project = match (args.project, args.name, args.cmd) {
                (Some(project), None, None) => project,
                (None, Some(name), Some(command)) => Project::from_cmd(name, command),
                _ => unreachable!("Only one of project or command should be specified"),
            };
            run(&config, project)?
        }
        SubCommands::Status(args) => status(&config, args)?,
        SubCommands::List(args) => list(&config, args)?,
        SubCommands::Logs(args) => logs(&config, args)?,
    }

    Ok(())
}
