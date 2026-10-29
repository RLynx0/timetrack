# timetrack

`timetrack` is a command-line time tracker. It records activity start and end
events, supports hierarchical activities, and can display summaries or generate
configured accounting output.

## Getting started

Create a configuration file with the guided prompts:

```sh
timetrack make-config
```

Define an activity with its work-breakdown-structure (WBS) value, then start
tracking it:

```sh
timetrack activity set work/project "I.12345678.01.01"
timetrack start work/project
```

Switch to the built-in `idle` activity to stop tracking work while leaving time
tracking active. Use `end` to finish the current entry:

```sh
timetrack start idle
timetrack end
```

Inspect the current activity or recent entries:

```sh
timetrack show
timetrack show entries collapsed --range 1week
timetrack show attendance --range 2026-01
```

## Commands

| Command | Purpose |
| --- | --- |
| `start [ACTIVITY]` | Start tracking an activity; ends the previous activity. Defaults to `idle`. |
| `end` | End the currently tracked activity. |
| `show [MODE...]` | Display entries, collapsed activity summaries, attendance, or total time. |
| `edit [entries\|config\|activities]` | Open a work file in an editor. |
| `generate` | Generate accounting output from the configured range. |
| `activity set NAME WBS` | Define or update an activity; missing categories are created. |
| `activity ls [CATEGORY]` | List activities, optionally recursively or collapsed. |
| `activity mv FROM TO` | Move or rename an activity or category. |
| `activity rm NAME` | Remove an activity; existing log entries are retained. |
| `list-attendance-types` | List configured attendance types. |
| `make-config` | Create a configuration file with guided prompts. |

Run `timetrack --help` or `timetrack <command> --help` for all options.

### Show modes and ranges

`show` accepts any combination of these modes:

- `entries`: individual tracked entries (the default)
- `collapsed`: grouped activity totals
- `attendance`: daily attendance periods and adjusted hours
- `time`: total tracked time

The `--range` option selects how much data to show:

- `0` (default): the active entry only
- A count such as `3`: the last three completed results, plus the active result
- Relative periods such as `2weeks`, `1month`, `day`, or `hour`
- A date bucket such as `2026-05` or `2026-05-12`
- A range such as `2026-01-01..2026-03-31`, `3m..1m`, or `..`

Relative periods include the current calendar bucket. Ranges use inclusive
calendar buckets. See `timetrack show --help` for the complete syntax and
examples. `generate --range` accepts the same syntax and overrides
`generate.range` in the configuration.

Pass `--machine-readable` (`-m`) to `show`, `activity ls`, or
`list-attendance-types` for tab-separated output rather than formatted tables.
`generate --stdout` prints generated output instead of saving it.

## Files and configuration

By default, configuration and activity definitions are stored in
`~/.config/timetrack/` (or `$XDG_CONFIG_HOME/timetrack/`). The configuration is
`config.toml`; the activity definitions are in `activities`. The activity log
is stored separately in `~/.local/share/timetrack/entries` (or
`$XDG_DATA_HOME/timetrack/entries`).

`TIMETRACK_HOME` overrides the config/activity directory, and
`TIMETRACK_DATA_HOME` overrides the entries directory. These variables take
precedence over the corresponding XDG locations. Directories are created when
the program writes to them.

Global CLI flags are accepted before or after subcommands. A flag overrides its
corresponding environment variable; directory fallbacks still prefer the
timetrack directory over the XDG base over `HOME`.
For example, `timetrack --config-home ./config show --data-home ./data`
reads `./config/config.toml` and `./data/entries`.

The configuration contains employee/accounting fields, a default attendance
type, an attendance-type map, and a `[generate]` section. `make-config` creates
a starting configuration; `timetrack edit config` opens it for changes. The
included [`assets/default_config.toml`](assets/default_config.toml) shows the
available fields and a sample set of attendance types.

The `[generate]` section controls the output destination, filename format,
column keys and values, delimiter, and default range. Format strings interpolate
variables using `$name` or `${name}` syntax. Available file-name variables are
`employee_name`, `employee_number`, `cost_center`, `performance_type`,
`accounting_cycle`, `year`, `month`, and `day`. Row-value variables are `year`,
`month`, `day`, `hours`, `minutes`, `seconds`, `attendance_type`,
`description`, and `wbs`.

Activity definitions have the form `path/to/activity<TAB>WBS<TAB>description`.
The description is optional and is used as the default when starting that
activity. `activity set` is the recommended way to create or update entries.

## Environment variables

| Variable | Global flag | Effect |
| --- | --- | --- |
| `TIMETRACK_HOME` | `--config-home <DIR>` | Config and activity definitions directory; overrides the XDG config location. |
| `TIMETRACK_DATA_HOME` | `--data-home <DIR>` | Entries directory; overrides the XDG data location. |
| `XDG_CONFIG_HOME` | None | Base for config files; defaults to `~/.config`. |
| `XDG_DATA_HOME` | None | Base for data files; defaults to `~/.local/share`. |
| `HOME` | None | Used to construct the default XDG locations if those are unset. |
| `EDITOR` | `--editor <EDITOR>` / `-e` | Editor used by `edit`; defaults to `vi`. |
| `MAX_TABLE_WIDTH` | `--max-table-width <MAX_TABLE_WIDTH>` | Unsigned integer maximum table width in terminal columns. If unset, table width follows the terminal when stdout is a TTY; redirected output is unbounded. If too small to fit the minimum column widths, columns are reduced to at most 4 columns each, so the table may still exceed the limit. |
| `FORCE_COLOR` | `--force-color` | Forces colored output, taking precedence over `NO_COLOR` / `--no-color`. `0` and `false` do not force color; other values (including empty) do. |
| `NO_COLOR` | `--no-color` | Disables colors unless forced. Any value except `0` (including empty) disables colors. |

`MAX_TABLE_WIDTH` also applies to redirected table output when explicitly set.
Tables may truncate cell contents to fit the configured width.
Invalid widths are reported as CLI argument errors.

## Build

With Rust installed, build and test using Cargo:

```sh
cargo build --release
cargo test
```

The Nix flake also provides a package and a development shell with Rust tooling.
