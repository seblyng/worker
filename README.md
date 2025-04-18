# worker

A command line utility for UNIX systems to run programs in the background,
while also being able to print out logs, status, stop and restart the
processes

This is mostly implemented for myself as a convenient script. I am using `libc`
to be able to fork processes and run them in the background. I cannot guarantee
that this is able to always have control of the running processes, and that
zombie processes wont happen, so use at your own risk :)

## Install

```sh
cargo install --git https://github.com/seblyng/worker
```

## Setup

To setup, create a `.worker.toml` in the root of where the projects are
located. For example, for one of my projects
[foodie](`https://github.com/seblyng/foodie`), a config file can look like this.

NOTE: To get the logs, I am piping stderr and stdout to a file, and a lot of
programs are suppressing ansi color codes when it is not a TTY. Because of
this, you might not see color in the logs when running a command. Please refer
to the docs for the program you are trying to run, to be able to pipe the
output with color codes. For example, with Rust, you are able to pass `--color
always` to `cargo` for it to not suppress the color codes when piping to a file

```toml
[[project]]
name = "frontend"
command = "trunk --color always serve"
cwd = "/Users/sebastian/projects/foodie/frontend"
envs = { CARGO_TERM_COLOR = "always" }
display = "Foodie Frontend"
group = [ "foodie" ]

[[project]]
name = "backend"
command = "cargo watch -x 'run --color always'"
cwd = "/Users/sebastian/projects/foodie/backend"
display = "Foodie Backend"
group = [ "foodie" ]
```

## How to run

```
Usage: worker <COMMAND>

Commands:
  start    Start the specified project(s). E.g. `worker start foo bar`
  stop     Stop the specified project(s). E.g. `worker stop foo bar`
  restart  Restart the specified project(s). E.g. `worker restart foo bar` (Same as running stop and then start)
  logs     Print out logs for the specified project
  status   Print out a status of which projects is running
  list     Print out a list of available projects to run
  run      Runs the project in the foreground
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

For the commands that accepts multiple args, you can use both `name` and `group` as an arg. If using `group`, it will start/stop/restart
all the projects within the same group (for example `worker start foodie` is the same as `worker start frontend backend`)
