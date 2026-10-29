use clap::Parser;
use color_eyre::eyre::{Context, Result};

use crate::{cli::Cli, environment::Environment};

mod activity_commands;
mod activity_entry;
mod activity_range;
mod cli;
mod config;
mod config_commands;
mod entry_commands;
mod environment;
mod files;
mod format_string;
mod printable;
mod style;
mod time_window;
mod trackable;

const NONE_PRINT_VALUE: &str = "--";

fn main() -> Result<()> {
    color_eyre::install()?;
    let opts = Cli::parse();
    opts.environment.apply_color();
    handle_ttr_command(&opts)
}

fn handle_ttr_command(cli: &Cli) -> Result<()> {
    let env = &cli.environment;
    match &cli.command {
        cli::TtrCommand::Start(start_opts) => {
            entry_commands::start_activity(start_opts, env).wrap_err("failed to start tracking")
        }
        cli::TtrCommand::End(end_opts) => {
            entry_commands::end_activity(end_opts, env).wrap_err("failed to end tracking")
        }
        cli::TtrCommand::Show(show_opts) => {
            entry_commands::show_activities(show_opts, env).wrap_err("failed to show activity")
        }
        cli::TtrCommand::Edit(edit_opts) => {
            entry_commands::handle_edit(edit_opts, env).wrap_err("failed to open entry file")
        }
        cli::TtrCommand::Generate(generate_opts) => {
            entry_commands::handle_generate(generate_opts, env)
                .wrap_err("failed to generate output")
        }
        cli::TtrCommand::Activity(activity_opts) => handle_activity_command(activity_opts, env),
        cli::TtrCommand::ListAttendanceTypes(list_opts) => {
            config_commands::list_attendance_types(list_opts, env)
        }
        cli::TtrCommand::MakeConfig => config_commands::make_guided_config(env),
    }
}

fn handle_activity_command(
    activity_command: &cli::ActivityCommand,
    env: &Environment,
) -> Result<()> {
    match activity_command {
        cli::ActivityCommand::Set(opts) => activity_commands::set_activity(opts, env)
            .wrap_err_with(|| format!("failed to set activity '{}'", opts.name)),
        cli::ActivityCommand::Rm(opts) => activity_commands::remove_activity(opts, env)
            .wrap_err_with(|| format!("failed to remove activity '{}'", opts.name)),
        cli::ActivityCommand::Mv(opts) => activity_commands::move_activity(opts, env)
            .wrap_err_with(|| format!("failed to move activity '{}'", opts.from)),
        cli::ActivityCommand::Ls(opts) => activity_commands::list_activities(opts, env)
            .wrap_err_with(|| match &opts.name {
                Some(n) => format!("failed to list activities in {n}"),
                None => String::from("failed to list activities"),
            }),
    }
}
