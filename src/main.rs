#![allow(unused)] // TODO: Remove this when more things are implemented

use std::{
    fs,
    io::{Write, stdin, stdout},
    path::Path,
    process::exit,
    str::FromStr,
};

use chrono::Local;
use clap::Parser;
use color_eyre::eyre::Result;
use rev_lines::RawRevLines;

use crate::{
    config::Config,
    entry::ActivityEntry,
    opt::{Opt, last_value::LastValue},
};

mod config;
mod entry;
mod files;
mod format_string;
mod opt;

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
        opt::TtrCommand::Start(opts) => handle_start(opts),
        opt::TtrCommand::End(opts) => end_activity(opts),
        opt::TtrCommand::Show(opts) => show_entries(opts),
        opt::TtrCommand::Generate(_) => todo!(),
        opt::TtrCommand::Activity(_) => todo!(),
    }
}

macro_rules! verbose_print_pretty {
    ($cond:expr => [$($k:expr => $v:expr,)+]) => {
        if $cond {
            $(
                {
                    let s = $v.to_string();
                    let s = s.trim();
                    (!s.is_empty()).then(|| println!(
                    "-> \u{001b}[34m{:12}\u{001b}[0m: {}",
                    $k, $v));
                }
            )+
        }
    };
}

fn handle_start(start_opts: &opt::Start) -> Result<()> {
    let config = &get_config()?;
    let activity_name: &str = &start_opts.activity;

    let wbs = resolve_wbs(activity_name)?;

    let last_entry = get_last_state_entry(&files::get_entry_file_path()?)?;
    let last_attendance = last_entry.as_ref().and_then(|e| e.attendance_type());
    let attendance = start_opts
        .attendance
        .as_deref()
        .or(last_attendance)
        .unwrap_or(&config.default_attendance);

    let description = start_opts
        .description
        .as_deref()
        .map(sanitize_description)
        .unwrap_or_default();

    let entry = ActivityEntry::new_start(activity_name, attendance, &wbs, &description);
    write_entry(&entry)?;

    if let Some(ActivityEntry::Start(last_start)) = last_entry.as_ref() {
        let last_name = last_start.name();
        println!("Stopped tracking \u{001B}[31m'{last_name}'\u{001b}[0m");
    }
    println!("Started tracking \u{001B}[32m'{activity_name}'\u{001B}[0m");

    let timestamp = entry.time_stamp();
    verbose_print_pretty! {
        start_opts.verbose => [
            "Description" => description,
            "Attendance" => attendance,
            "WBS" => wbs,
            "Date" => timestamp.format("%Y-%m-%d"),
            "Time" => timestamp.format("%H:%M:%S"),
        ]
    };

    Ok(())
}

fn resolve_wbs(activity_name: &str) -> Result<String> {
    if activity_name == BUILTIN_ACTIVITY_IDLE {
        return Ok(IDLE_WBS_SENTINEL.to_owned());
    }

    Err(color_eyre::eyre::format_err!(
        "Activity {activity_name} does not exist."
    ))
}

fn sanitize_description(description: &str) -> String {
    description.replace("\t", "    ").replace("\n", " -- ")
}

fn end_activity(end_opts: &opt::End) -> Result<()> {
    let last_entry = get_last_state_entry(&files::get_entry_file_path()?)?;

    if let Some(ActivityEntry::Start(last_start)) = last_entry.as_ref() {
        let last_name = last_start.name();
        println!("Stopped tracking activity \u{001B}[31m'{last_name}'\u{001b}[0m");
    } else {
        println!("Stopped tracking time");
    }

    let entry = ActivityEntry::new_end();
    let timestamp = entry.time_stamp();
    verbose_print_pretty!(
        end_opts.verbose => [
            "Date" => timestamp.format("%Y-%m-%d"),
            "Time" => timestamp.format("%H:%M:%S"),
        ]
    );

    write_entry(&entry)
}

fn write_entry(entry: &ActivityEntry) -> Result<()> {
    let path = files::get_entry_file_path()?;
    if !fs::exists(&path)?
        && let Some(p) = path.parent()
    {
        fs::create_dir_all(p)?
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    writeln!(&mut file, "{entry}")?;
    Ok(())
}

fn show_entries(show_opts: &opt::Show) -> Result<()> {
    if matches!(show_opts.last, LastValue::SingleEntries(1)) {
        let entry = get_last_state_entry(&files::get_entry_file_path()?)?;
        match entry {
            None => println!("You have not recorded any data yet"),
            Some(ActivityEntry::End(_)) => {
                println!("You are not tracking any activity")
            }
            Some(ActivityEntry::Start(entry)) => {
                println!(
                    "Tracking activity \u{001B}[32m'{}'\u{001B}[0m",
                    entry.name()
                );

                let delta = Local::now() - entry.time_stamp();
                verbose_print_pretty! {
                    true => [
                        "Description" => entry.description(),
                        "Attendance" => entry.attendance(),
                        "WBS" => entry.wbs(),
                        "Tracked for" => delta,
                    ]
                };
            }
        }
    }
    Ok(())
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
