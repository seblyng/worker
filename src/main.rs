use std::time::{Duration, Instant};

use anyhow::anyhow;
use clap::{command, Parser};
use config::WorkerConfig;
use itertools::Itertools;
use project::Project;

pub mod config;
pub mod libc;
pub mod project;

fn logs(config: &WorkerConfig, args: LogsArgs) -> Result<(), anyhow::Error> {
    if !args.project.is_running(config)? {
        return Err(anyhow!("{} is not running", args.project));
    }

    let mut cmd = std::process::Command::new("tail");

    if args.follow {
        cmd.arg("-f");
    }

    let mut child = cmd
        .args(["-n", &args.number.to_string()])
        .arg(config.log_file(&args.project))
        .spawn()?;

    child.wait()?;

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

fn stop(config: &WorkerConfig, projects: Vec<Project>) -> Result<(), anyhow::Error> {
    let (running, not_running) = config.partition_projects(projects)?;

    for project in running.iter() {
        project.stop()?;
    }

    for project in not_running {
        eprintln!("Cannot stop project not running: {}", project);
    }

    let timeout = Duration::new(5, 0);
    let start = Instant::now();

    while Instant::now().duration_since(start) < timeout {
        let (still_running, _) = config.partition_projects(running.clone())?;
        if still_running.is_empty() {
            return Ok(());
        }
    }

    let (still_running, _) = config.partition_projects(running)?;
    for p in still_running {
        eprintln!("Was not able to stop {}", p);
    }

    Ok(())
}

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

fn run(config: &WorkerConfig, project: Project) -> Result<(), anyhow::Error> {
    project.start_dependencies(config)?;

    project.run()?;

    Ok(())
}

fn restart(config: &WorkerConfig, projects: Vec<Project>) -> Result<(), anyhow::Error> {
    let (projects, filtered) = config.partition_projects(projects)?;
    let projects: Vec<Project> = projects.into_iter().map(|p| p.into()).collect();

    for project in filtered {
        eprintln!("Cannot restart project not running: {}", project);
    }

    stop(config, projects.clone())?;
    start(config, projects)?;

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

#[derive(Clone, Debug, Eq, PartialEq)]
enum ActionArg {
    Project(Project),
    Group(Vec<Project>),
}

#[derive(Debug, Parser)]
struct ActionArgs {
    projects: Vec<ActionArg>,
}

#[derive(Debug, Parser)]
struct RunArgs {
    project: Project,
}

#[derive(Debug, Parser)]
struct LogsArgs {
    project: Project,
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
    Start(ActionArgs),
    /// Stop the specified project(s). E.g. `worker stop foo bar`
    Stop(ActionArgs),
    /// Restart the specified project(s). E.g. `worker restart foo bar` (Same as running stop and then start)
    Restart(ActionArgs),
    /// Print out logs for the specified project.
    Logs(LogsArgs),
    /// Print out a status of which projects is running
    Status(StatusArgs),
    /// Print out a list of available projects to run
    List(ListArgs),
    /// Runs the project in the foreground
    Run(RunArgs),
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
        SubCommands::Start(args) => start(&config, unique(args.projects))?,
        SubCommands::Stop(args) => stop(&config, unique(args.projects))?,
        SubCommands::Restart(args) => restart(&config, unique(args.projects))?,
        SubCommands::Logs(args) => logs(&config, args)?,
        SubCommands::Status(args) => status(&config, args)?,
        SubCommands::List(args) => list(&config, args)?,
        SubCommands::Run(args) => run(&config, args.project)?,
    }

    Ok(())
}
