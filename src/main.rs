#![allow(unused)] // TODO: Remove this when more things are implemented

use std::{
    fs,
    io::{Write, stdin, stdout},
    path::{Path, PathBuf},
    process::exit,
    str::FromStr,
};

use clap::Parser;
use color_eyre::eyre::Result;
use rev_lines::RawRevLines;

use crate::{config::Config, entry::ActivityEntry, opt::Opt};

mod config;
mod entry;
mod files;
mod format_string;
mod opt;

const IDLE_WBS_SENTINEL: &str = "Idle";
const BUILTIN_ACTIVITY_IDLE: &str = "Idle";
const BUILTIN_ACTIVITY_INTERN: &str = "Intern";

fn main() {
    let opt = Opt::parse();
    if let Err(err) = handle_ttr_command(&opt) {
        eprintln!("{err}");
        exit(1)
    }
}

fn handle_ttr_command(opt: &Opt) -> Result<()> {
    let cfg_path = opt.config.as_ref();
    match &opt.command {
        opt::TtrCommand::Start(opts) => start_activity(&get_config(cfg_path)?, opts),
        opt::TtrCommand::Idle(opts) => start_idle(&get_config(cfg_path)?, opts),
        opt::TtrCommand::End(opts) => end_activity(&get_config(cfg_path)?, opts),
        opt::TtrCommand::Activity(_) => todo!(),
        opt::TtrCommand::Generate(_) => todo!(),
    }
}

macro_rules! verbose_print_pretty {
    ($cond:expr => [$($k:expr => $v:expr,)+]) => {
        if $cond {
            $(
                (!$v.to_string().trim().is_empty()).then(|| println!(
                    "-> \u{001b}[34m{:12}\u{001b}[0m: {}",
                    $k, $v
                ));
            )+
        }
    };
}

fn start_activity(config: &Config, start_opts: &opt::Start) -> Result<()> {
    let activity_name = &start_opts.activity;

    let last_entry = get_last_state_entry(&files::get_entry_file_path()?)?;
    let attendance = &match last_entry {
        Some(ActivityEntry::Start(start_entry)) => start_entry.attendance().to_owned(),
        _ => config.default_attendance.to_owned(),
    };

    let wbs = "I.03099999.99";

    let descr = match &start_opts.description {
        Some(s) => s.replace("\t", "    ").replace("\n", " -- "),
        None => String::new(),
    };

    let entry = ActivityEntry::new_start(activity_name, attendance, wbs, &descr);
    println!("Started tracking activity \u{001B}[32m'{activity_name}'\u{001B}[0m");

    let timestamp = entry.time_stamp();
    verbose_print_pretty! {
        start_opts.verbose => [
            "Description" => descr,
            "Attendance" => attendance,
            "WBS" => wbs,
            "Date" => timestamp.format("%Y-%m-%d"),
            "Time" => timestamp.format("%H:%M:%S"),
        ]
    };
    Ok(())
}

fn start_idle(config: &Config, idle_opts: &opt::Idle) -> Result<()> {
    let activity_name = BUILTIN_ACTIVITY_IDLE;

    let last_entry = get_last_state_entry(&files::get_entry_file_path()?)?;
    let attendance = &match last_entry {
        Some(ActivityEntry::Start(start_entry)) => start_entry.attendance().to_owned(),
        _ => config.default_attendance.to_owned(),
    };

    let wbs = IDLE_WBS_SENTINEL;

    let descr = match &idle_opts.description {
        Some(s) => s.replace("\t", "    ").replace("\n", " -- "),
        None => String::new(),
    };

    let entry = ActivityEntry::new_start(activity_name, attendance, wbs, &descr);
    println!("Started tracking activity \u{001B}[32m'{activity_name}'\u{001B}[0m");

    let timestamp = entry.time_stamp();
    verbose_print_pretty! {
        idle_opts.verbose => [
            "Description" => descr,
            "Attendance" => attendance,
            "WBS" => wbs,
            "Date" => timestamp.format("%Y-%m-%d"),
            "Time" => timestamp.format("%H:%M:%S"),
        ]
    };
    Ok(())
}

fn end_activity(config: &Config, end_opts: &opt::End) -> Result<()> {
    let entry = ActivityEntry::new_end();
    println!("Stopped tracking time");

    let timestamp = entry.time_stamp();
    verbose_print_pretty!(
        end_opts.verbose => [
            "Date" => timestamp.format("%Y-%m-%d"),
            "Time" => timestamp.format("%H:%M:%S"),
        ]
    );
    Ok(())
}

fn get_config(custom_path: Option<&PathBuf>) -> Result<Config> {
    let config_path = match custom_path {
        None => &files::default_config_path()?,
        Some(p) => p,
    };
    if fs::exists(config_path)? {
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
        fs::write(config_path, config_str)?;
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

fn get_last_state_entry(path: &Path) -> Result<Option<ActivityEntry>> {
    if !fs::exists(path)? {
        return Ok(None);
    }
    let file = fs::File::open(path)?;
    let mut rev_lines = RawRevLines::new(file);
    match rev_lines.next() {
        Some(res) => {
            let entry = &String::from_utf8(res?)?;
            Ok(Some(ActivityEntry::from_str(entry)?))
        }
        None => Ok(None),
    }
}
