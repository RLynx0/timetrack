use std::{
    fs,
    io::{Write, stdin, stdout},
};

use clap::Parser;
use color_eyre::{
    Section,
    eyre::{Context, Result},
};

use crate::{cli::Cli, config::Config};

mod activity_commands;
mod activity_entry;
mod activity_range;
mod cli;
mod config;
mod entry_commands;
mod files;
mod format_string;
mod printable;
mod trackable;

const NONE_PRINT_VALUE: &str = "--";

fn main() -> Result<()> {
    color_eyre::install()?;
    let opts = Cli::parse();
    handle_ttr_command(&opts)
}

fn handle_ttr_command(opts: &Cli) -> Result<()> {
    match &opts.command {
        cli::TtrCommand::Start(opts) => {
            entry_commands::start_activity(opts).wrap_err("failed to start tracking")
        }
        cli::TtrCommand::End(opts) => {
            entry_commands::end_activity(opts).wrap_err("failed to end tracking")
        }
        cli::TtrCommand::Show(opts) => {
            entry_commands::show_activities(opts).wrap_err("failed to show activitiy")
        }
        cli::TtrCommand::Edit(_) => {
            entry_commands::open_entry_file().wrap_err("failed to open entry file")
        }
        cli::TtrCommand::Generate(opts) => {
            entry_commands::handle_generate(opts).wrap_err("failed to generate output")
        }
        cli::TtrCommand::Activity(opts) => handle_activity_command(opts),
        cli::TtrCommand::ListAttendanceTypes(opts) => list_attendance_types(opts),
        cli::TtrCommand::GenerateConfig => make_guided_config(),
    }
}

fn handle_activity_command(activity_command: &cli::ActivityCommand) -> Result<()> {
    match activity_command {
        cli::ActivityCommand::Set(opts) => activity_commands::set_activity(opts)
            .wrap_err_with(|| format!("failed to set activity '{}'", opts.name)),
        cli::ActivityCommand::Rm(_) => todo!(),
        cli::ActivityCommand::Mv(_) => todo!(),
        cli::ActivityCommand::Ls(opts) => {
            activity_commands::list_activities(opts).wrap_err_with(|| match &opts.name {
                Some(n) => format!("failed to list activities in {n}"),
                None => String::from("failed to list activities"),
            })
        }
    }
}

fn list_attendance_types(list_opts: &cli::ListAttendanceTypes) -> Result<()> {
    let config = get_config()?;
    let mut list = config.attendance_types.into_iter().collect::<Vec<_>>();
    list.sort_by(|(_, va), (_, vb)| va.cmp(vb));
    if list_opts.machine_readable {
        for (number, hint) in list {
            println!("{number}\t{hint}")
        }
    } else {
        print_smart_list!(list);
    }
    Ok(())
}

fn get_config() -> Result<Config> {
    let config_path = files::get_main_config_path()?;
    if fs::exists(&config_path)? {
        let config_str = fs::read_to_string(config_path)?;
        Ok(toml::from_str(&config_str)?)
    } else {
        Err(color_eyre::eyre::format_err!(
            "{config_path:?} does not exist"
        ))
        .wrap_err("Failed to load configuration")
        .with_note(|| "`generate-config` can help you create a configuration file")
    }
}

fn make_guided_config() -> Result<()> {
    let config_path = files::get_main_config_path()?;
    let default = toml::from_str::<Config>(include_str!("../assets/default_config.toml"))
        .expect("Default config must be valid");

    println!("Let me guide you through creating a configuration file!");
    let employee_name = get_input_string("Your Name")?;
    let employee_number = get_input_string("Your emplyee id")?;
    let cost_center = get_input_string("Your cost center")?;
    let performance_type = get_input_string("Your performance type")?;
    let accounting_cycle = get_input_string("Your accounting cycle")?;

    println!("\nOkay, that's it!");
    let default_attendance = &default.default_attendance;
    println!("\nI have set your default attendance type to {default_attendance}.",);
    println!("This is probably what you want, but in case it isn't -");
    println!("You can always manually edit the generated config file.");
    println!("There are a few more options there that were also set automatically!");

    let config = Config {
        employee_name,
        employee_number,
        cost_center,
        performance_type,
        accounting_cycle,
        ..default
    };

    let config_str = toml::to_string(&config)?;
    if let Some(p) = config_path.parent() {
        fs::create_dir_all(p)?;
    }
    fs::write(&config_path, config_str)?;
    println!("Your config has been saved to {config_path:?}.");
    Ok(())
}

fn get_input_string(query: &str) -> Result<String> {
    let mut input = String::new();
    while input.trim().is_empty() {
        print!("{query}: ");
        stdout().flush()?;
        stdin().read_line(&mut input)?;
    }
    Ok(input.trim().into())
}
