# worker

A small UNIX CLI to run projects in the background, view logs, check status,
and stop or restart them.

## Install

```sh
cargo install --git https://github.com/seblyng/worker
```

## Setup

Create `.worker.toml` in your project root. Run `worker` from that directory or
any subdirectory:

```toml
[[project]]
name = "database"
command = ["docker", "compose", "up", "-d", "db"]
cwd = "/absolute/path/to/app"
display = "Database" # Optional label
group = ["app"]

[project.lifecycle]
stop = ["docker", "compose", "stop", "db"]
logs = ["docker", "compose", "logs", "db"]
status = ["sh", "-c", "docker compose ps --status running -q db | grep -q ."]
```

`name`, `command`, and `cwd` are required. Optional `envs = { KEY = "value" }`
sets environment variables; `dependencies = ["other-project"]` starts dependencies
in the background first. Commands are argument arrays; use `["sh", "-c", "..."]`
for shell syntax.

## Lifecycle

For services that manage their own processes, use `[project.lifecycle]` as above.
Each override is optional and uses the project's `cwd` and `envs`:

- `stop` replaces the default process-group signal (`SIGINT`, configurable via `stop_signal`).
- `logs` replaces the log reader; `--follow` appends `-f`, while `--lines` is ignored.
- `status` determines whether the project is running: exit code 0 means running.

## Usage

```sh
worker start database      # Background; accepts multiple project or group names
worker stop app            # Stop running projects in a group
worker restart database    # Stop, then start a running project
worker run database        # Foreground (for commands that stay attached)
worker logs database -f    # Follow logs (-n / --lines sets line count)
worker status              # Running projects (-q for names only)
worker list                # Configured projects (-q for names only)
worker start --name task --cmd 'sleep 60'  # One-off command; also works with run
```

State and logs live in `.worker/state` and `.worker/log`. Background stdout and
stderr share a log file, overwritten on each start. Force color in your program
if needed (for Cargo, `CARGO_TERM_COLOR = "always"`). Use `worker <command> --help`
for options.
