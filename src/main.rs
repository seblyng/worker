use std::time::{Duration, Instant};

use clap::{
    ArgGroup, CommandFactory, FromArgMatches, Parser,
    builder::{PossibleValuesParser, TypedValueParser},
};
use itertools::Itertools;
use worker::{
    ActionArg, ActionArgRunning,
    config::WorkerConfig,
    project::{Project, RunningProject, WorkerProject},
};

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

fn stop(projects: Vec<RunningProject>) -> Result<(), anyhow::Error> {
    for project in projects.iter() {
        project.stop()?;
    }

    let timeout = Duration::new(5, 0);
    let start = Instant::now();

    while Instant::now().duration_since(start) < timeout {
        let still_running = projects.iter().filter(|p| p.is_running());
        if still_running.count() == 0 {
            return Ok(());
        }
    }

    for p in projects.iter().filter(|p| p.is_running()) {
        eprintln!("Was not able to stop {}", p);
    }

    Ok(())
}

fn restart(config: &WorkerConfig, projects: Vec<RunningProject>) -> Result<(), anyhow::Error> {
    stop(projects.clone())?;
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
    let mut child = if let Some(ref lifecycle) = args.project.lifecycle
        && let Some(ref logs) = lifecycle.logs
    {
        std::process::Command::new(&logs[0])
            .args(&logs[1..])
            .args(args.follow.then_some("-f"))
            .envs(args.project.envs.clone().unwrap_or_default())
            .current_dir(&args.project.cwd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?
    } else {
        std::process::Command::new("tail")
            .args(["-n", &args.number.to_string()])
            .args(args.follow.then_some("-f"))
            .arg(config.log_file(&args.project))
            .spawn()?
    };

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

fn project_names<T: WorkerProject>(projects: &[T]) -> Vec<String> {
    projects
        .iter()
        .flat_map(|p| std::iter::once(p.name()).chain(p.groups().iter().map(String::as_str)))
        .unique()
        .map(str::to_owned)
        .collect()
}

fn main() -> Result<(), anyhow::Error> {
    let config = WorkerConfig::new();
    let mut cmd = Cli::command();

    // Keep help and CLI syntax errors available even without a valid config.
    if let Ok(config) = &config {
        let names = project_names(&config.projects);
        let parser = PossibleValuesParser::new(&names).try_map(|name| name.parse::<ActionArg>());
        cmd = cmd.mut_subcommand("start", |cmd| {
            cmd.mut_arg("projects", |arg| arg.value_parser(parser))
        });

        let projects = project_names(&config.running()?.collect::<Vec<_>>());
        let names: Vec<_> = names.iter().cloned().chain(projects).unique().collect();
        let parser = PossibleValuesParser::new(names).try_map(|n| n.parse::<ActionArgRunning>());

        for subcommand in ["stop", "restart"] {
            cmd = cmd.mut_subcommand(subcommand, |cmd| {
                cmd.mut_arg("projects", |arg| arg.value_parser(parser.clone()))
            });
        }

        let names: Vec<_> = config.projects.iter().map(|p| p.name.clone()).collect();
        let parser = PossibleValuesParser::new(&names).try_map(|name| name.parse::<Project>());

        cmd = cmd.mut_subcommand("run", |cmd| {
            cmd.mut_arg("project", |arg| arg.value_parser(parser))
        });
    }

    let matches = cmd.get_matches();
    let cli = Cli::from_arg_matches(&matches)?;
    let config = config?;

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
