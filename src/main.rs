#![allow(unused)] // TODO: Remove this when more things are implemented

use std::{
    fs,
    io::{Write, stdin, stdout},
    process::exit,
};

use clap::Parser;
use color_eyre::eyre::Result;

use crate::{config::Config, opt::Opt};

mod activity;
mod activity_range;
mod config;
mod entry_commands;
mod files;
mod format_string;
mod generate;
mod opt;
mod printable;

const IDLE_WBS_SENTINEL: &str = "Idle";
const BUILTIN_ACTIVITY_IDLE: &str = "Idle";

fn main() {
    let opt = Opt::parse();
    if let Err(err) = handle_ttr_command(&opt) {
        eprintln!("{err}");
        exit(1)
    }
}

fn handle_ttr_command(opt: &Opt) -> Result<()> {
    match &opt.command {
        opt::TtrCommand::Start(opts) => entry_commands::start_activity(opts),
        opt::TtrCommand::End(opts) => entry_commands::end_activity(opts),
        opt::TtrCommand::Show(opts) => entry_commands::show_activities(opts),
        opt::TtrCommand::Edit(_) => entry_commands::open_entry_file(),
        opt::TtrCommand::Generate(_) => todo!(),
        opt::TtrCommand::Activity(opts) => handle_activity_command(opts),
    }
}

fn handle_activity_command(activity_command: &opt::ActivityCommand) -> Result<()> {
    match activity_command {
        opt::ActivityCommand::New(_) => todo!(),
        opt::ActivityCommand::Rm(_) => todo!(),
        opt::ActivityCommand::Ls(_) => todo!(),
    }
}

fn resolve_wbs(activity_name: &str) -> Result<String> {
    if activity_name == BUILTIN_ACTIVITY_IDLE {
        return Ok(IDLE_WBS_SENTINEL.to_owned());
    }

    Err(color_eyre::eyre::format_err!(
        "Activity {activity_name} does not exist."
    ))
}

fn get_config() -> Result<Config> {
    let config_path = files::get_main_config_path()?;
    if fs::exists(&config_path)? {
        let config_str = fs::read_to_string(config_path)?;
        Ok(toml::from_str(&config_str)?)
    } else {
        println!("I couldn't find the required config at {config_path:?}");
        println!("Let me guide you through creating your configuration!");
        let config = make_guided_config()?;
        let config_str = toml::to_string(&config)?;
        if let Some(p) = config_path.parent() {
            fs::create_dir_all(p)?;
        }
        fs::write(&config_path, config_str)?;
        println!("Saved generated configuration to {config_path:?}");
        println!("\n--------\n");
        Ok(config)
    }
}

fn make_guided_config() -> Result<Config> {
    let default = toml::from_str::<Config>(include_str!("./default_config.toml"))
        .expect("Default config must be valid");

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

    Ok(Config {
        employee_name,
        employee_number,
        cost_center,
        performance_type,
        accounting_cycle,
        ..default
    })
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
