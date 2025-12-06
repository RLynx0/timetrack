#![allow(unused)] // TODO: Remove this when more things are implemented

use std::{
    fs,
    io::{self, Write, stdin, stdout},
    path::Path,
    process::exit,
    rc::Rc,
    str::FromStr,
};

use chrono::{DateTime, Datelike, Local, TimeDelta, Timelike};
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
        opt::TtrCommand::Start(opts) => start_activity(opts),
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

fn start_activity(start_opts: &opt::Start) -> Result<()> {
    let config = &get_config()?;
    let activity_name: &str = &start_opts.activity;

    let wbs = resolve_wbs(activity_name)?;

    let last_entry = get_last_entry(&files::get_entry_file_path()?)?;
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
    let last_entry = get_last_entry(&files::get_entry_file_path()?)?;
    match last_entry.as_ref() {
        Some(ActivityEntry::Start(last_start)) => {
            let entry = ActivityEntry::new_end();
            write_entry(&entry)?;

            let stopped = last_start.name();
            println!("Stopped tracking \u{001B}[31m'{stopped}'\u{001b}[0m");
            let timestamp = entry.time_stamp();
            verbose_print_pretty!(
                end_opts.verbose => [
                    "Date" => timestamp.format("%Y-%m-%d"),
                    "Time" => timestamp.format("%H:%M:%S"),
                ]
            );
            Ok(())
        }
        _ => Err(color_eyre::eyre::format_err!(
            "You are not tracking any activity"
        )),
    }
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
    match &show_opts.last {
        LastValue::SingleEntries(1) => show_last_entry(),
        lval => show_multiple_entries(lval),
    }
}

fn show_last_entry() -> Result<()> {
    let entry = get_last_entry(&files::get_entry_file_path()?)?;
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
    Ok(())
}

fn show_multiple_entries(lval: &LastValue) -> Result<()> {
    let reversed_entries = match lval {
        LastValue::SingleEntries(n) => get_last_n_entries(&files::get_entry_file_path()?, *n)?,
        LastValue::Hours(h) => {
            let start_time = Local::now()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap()
                - TimeDelta::hours(*h as i64 - 1);
            get_entries_since(&files::get_entry_file_path()?, &start_time)?
        }
        LastValue::Days(d) => {
            let start_time = Local::now()
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap()
                - TimeDelta::days(*d as i64 - 1);
            get_entries_since(&files::get_entry_file_path()?, &start_time)?
        }
        LastValue::Months(m) => {
            let start_time = Local::now()
                .with_day(1)
                .unwrap()
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap();
            // TODO: subtract m months
            get_entries_since(&files::get_entry_file_path()?, &start_time)?
        }
    };

    if reversed_entries.is_empty() {
        println!("You have not recorded any data yet");
    } else {
        print_entry_table(reversed_entries.into_iter().rev());
    }

    Ok(())
}

fn print_entry_table(entries: impl IntoIterator<Item = ActivityEntry>) {
    let mut col_time: Vec<Rc<str>> = vec!["Timestamp".into()];
    let mut col_name: Vec<Rc<str>> = vec!["Activity".into()];
    let mut col_attendance: Vec<Rc<str>> = vec!["Attendance".into()];
    let mut col_wbs: Vec<Rc<str>> = vec!["WBS".into()];
    let mut col_description: Vec<Rc<str>> = vec!["Description".into()];

    for entry in entries {
        col_time.push(
            entry
                .time_stamp()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .into(),
        );
        match entry {
            ActivityEntry::Start(activity_start) => {
                col_name.push(activity_start.name().into());
                col_attendance.push(activity_start.attendance().into());
                col_wbs.push(activity_start.wbs().into());
                col_description.push(
                    match activity_start.description() {
                        "" => "--",
                        s => s,
                    }
                    .into(),
                );
            }
            ActivityEntry::End(_) => {
                col_name.push("--".into());
                col_attendance.push("--".into());
                col_wbs.push("--".into());
                col_description.push("--".into());
            }
        }
    }

    let time_width = col_time.iter().map(|s| s.chars().count()).max();
    let name_width = col_name.iter().map(|s| s.chars().count()).max();
    let attendance_width = col_attendance.iter().map(|s| s.chars().count()).max();
    let wbs_width = col_wbs.iter().map(|s| s.chars().count()).max();
    let description_width = col_description.iter().map(|s| s.chars().count()).max();

    println!(
        "| \u{001b}[34m{}\u{001b}[0m{} | \u{001b}[34m{}\u{001b}[0m{} | \u{001b}[34m{}\u{001b}[0m{} | \u{001b}[34m{}\u{001b}[0m{} | \u{001b}[34m{}\u{001b}[0m{} |",
        col_time[0],
        " ".repeat(time_width.unwrap_or_default() - col_time[0].chars().count()),
        col_name[0],
        " ".repeat(name_width.unwrap_or_default() - col_name[0].chars().count()),
        col_attendance[0],
        " ".repeat(attendance_width.unwrap_or_default() - col_attendance[0].chars().count()),
        col_wbs[0],
        " ".repeat(wbs_width.unwrap_or_default() - col_wbs[0].chars().count()),
        col_description[0],
        " ".repeat(description_width.unwrap_or_default() - col_description[0].chars().count()),
    );
    println!(
        "|-{}-|-{}-|-{}-|-{}-|-{}-|",
        "-".repeat(time_width.unwrap_or_default()),
        "-".repeat(name_width.unwrap_or_default()),
        "-".repeat(attendance_width.unwrap_or_default()),
        "-".repeat(wbs_width.unwrap_or_default()),
        "-".repeat(description_width.unwrap_or_default()),
    );

    for ((((t, n), a), w), d) in col_time
        .iter()
        .zip(col_name)
        .zip(col_attendance)
        .zip(col_wbs)
        .zip(col_description)
        .skip(1)
    {
        println!(
            "| {t}{} | {n}{} | {a}{} | {w}{} | {d}{} |",
            " ".repeat(time_width.unwrap_or_default() - t.chars().count()),
            " ".repeat(name_width.unwrap_or_default() - n.chars().count()),
            " ".repeat(attendance_width.unwrap_or_default() - a.chars().count()),
            " ".repeat(wbs_width.unwrap_or_default() - w.chars().count()),
            " ".repeat(description_width.unwrap_or_default() - d.chars().count()),
        )
    }
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

fn get_last_entry(path: &Path) -> Result<Option<ActivityEntry>> {
    get_last_n_entries(path, 1).map(|v| v.into_iter().next())
}

/// Fetch the last `count` entries from `path` in reversed order
fn get_last_n_entries(path: &Path, count: usize) -> Result<Vec<ActivityEntry>> {
    if !fs::exists(path)? {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path)?;
    RawRevLines::new(file)
        .take(count)
        .map(entry_from_byte_result)
        .collect::<Result<Vec<_>>>()
}

/// Fetch entries since `start_time` from `path` in reversed order
fn get_entries_since(path: &Path, start_time: &DateTime<Local>) -> Result<Vec<ActivityEntry>> {
    if !fs::exists(path)? {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    let file = fs::File::open(path)?;
    for line in RawRevLines::new(file) {
        let entry = entry_from_byte_result(line)?;
        if entry.time_stamp() < start_time {
            break;
        }
        entries.push(entry);
    }

    Ok(entries)
}

fn entry_from_byte_result(
    byte_result: std::result::Result<Vec<u8>, io::Error>,
) -> Result<ActivityEntry> {
    let entry_str = String::from_utf8(byte_result?)?;
    Ok(ActivityEntry::from_str(&entry_str)?)
}
