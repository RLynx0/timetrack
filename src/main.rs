#![allow(unused)] // TODO: Remove this when more things are implemented

use std::{
    env, fs,
    io::{self, Write, stdin, stdout},
    path::Path,
    process::{Command, exit},
    rc::Rc,
    str::FromStr,
};

use chrono::{DateTime, Datelike, Local, TimeDelta, Timelike};
use clap::Parser;
use color_eyre::eyre::{Result, format_err};
use rev_lines::RawRevLines;

use crate::{
    config::Config,
    entry::ActivityEntry,
    files::get_entry_file_path,
    opt::{Opt, activity_quantity::ActivityQuantity},
    table::{ColorOptions, Table},
};

mod config;
mod entry;
mod files;
mod format_string;
mod opt;
mod table;

const IDLE_WBS_SENTINEL: &str = "Idle";
const BUILTIN_ACTIVITY_IDLE: &str = "Idle";

const ANSII_RED: &str = "\u{001b}[31m";
const ANSII_GREEN: &str = "\u{001b}[32m";
const ANSII_BLUE: &str = "\u{001b}[34m";
const ANSII_RESET: &str = "\u{001b}[0m";

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
        opt::TtrCommand::Edit(opts) => open_entry_file(opts),
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
                    "-> {ANSII_BLUE}{:12}{ANSII_RESET}: {}",
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
        println!("Stopped tracking {ANSII_RED}'{last_name}'{ANSII_RESET}");
    }
    println!("Started tracking {ANSII_GREEN}'{activity_name}'{ANSII_RESET}");

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
            println!("Stopped tracking {ANSII_RED}'{stopped}'{ANSII_RESET}");
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
        ActivityQuantity::SingleEntries(1) => show_current_entry(),
        lval => show_multiple_entries(lval),
    }
}

fn show_current_entry() -> Result<()> {
    let entry = get_last_entry(&files::get_entry_file_path()?)?;
    match entry {
        None => println!("You have not recorded any data yet"),
        Some(ActivityEntry::End(_)) => {
            println!("You are not tracking any activity")
        }
        Some(ActivityEntry::Start(entry)) => {
            println!(
                "Tracking activity {ANSII_GREEN}'{}'{ANSII_RESET}",
                entry.name()
            );

            let delta = Local::now() - entry.time_stamp();
            verbose_print_pretty! {
                true => [
                    "Description" => entry.description(),
                    "Attendance" => entry.attendance(),
                    "WBS" => entry.wbs(),
                    "Tracked for" => format_time_delta(&delta),
                ]
            };
        }
    }
    Ok(())
}

fn format_time_delta(delta: &TimeDelta) -> String {
    let mut out = String::new();
    let days = delta.num_days();
    if days > 0 {
        out.push_str(&format!("{days}d "))
    }

    let rem = *delta - TimeDelta::days(days);
    let hours = rem.num_hours();
    if hours > 0 {
        out.push_str(&format!("{hours}h "))
    }

    let rem = rem - TimeDelta::hours(hours);
    let minutes = rem.num_minutes();
    if minutes > 0 {
        out.push_str(&format!("{minutes}m "))
    }

    let rem = rem - TimeDelta::minutes(minutes);
    let seconds = rem.num_seconds();
    out.push_str(&format!("{seconds}s"));

    out
}

fn show_multiple_entries(lval: &ActivityQuantity) -> Result<()> {
    let reversed_entries = match lval {
        ActivityQuantity::SingleEntries(n) => {
            get_last_n_entries(&files::get_entry_file_path()?, *n as usize)?
        }
        ActivityQuantity::Hours(h) => {
            let start_time = Local::now()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap()
                - TimeDelta::hours(*h);
            get_entries_since(&files::get_entry_file_path()?, &start_time)?
        }
        ActivityQuantity::Days(d) => {
            let start_time = Local::now()
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap()
                - TimeDelta::days(*d);
            get_entries_since(&files::get_entry_file_path()?, &start_time)?
        }
        ActivityQuantity::Weeks(w) => {
            let now = Local::now();
            let day_offset = now.weekday().num_days_from_monday();
            let start_time = now
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap()
                - TimeDelta::days(7 * w + day_offset as i64);
            get_entries_since(&files::get_entry_file_path()?, &start_time)?
        }
        ActivityQuantity::Months(m) => {
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
    let mut col_date: Vec<Rc<str>> = Vec::new();
    let mut col_start: Vec<Rc<str>> = Vec::new();
    let mut col_name: Vec<Rc<str>> = Vec::new();
    let mut col_attendance: Vec<Rc<str>> = Vec::new();
    let mut col_wbs: Vec<Rc<str>> = Vec::new();
    let mut col_description: Vec<Rc<str>> = Vec::new();
    let none_value: Rc<str> = "--".into();
    for entry in entries {
        let timestamp = entry.time_stamp();
        col_date.push(timestamp.format("%Y-%m-%d").to_string().into());
        col_start.push(timestamp.format("%H:%M:%S").to_string().into());
        match entry {
            ActivityEntry::Start(activity_start) => {
                col_name.push(activity_start.name().into());
                col_attendance.push(activity_start.attendance().into());
                col_wbs.push(activity_start.wbs().into());
                col_description.push(match activity_start.description() {
                    "" => none_value.clone(),
                    s => s.into(),
                });
            }
            ActivityEntry::End(_) => {
                col_name.push(none_value.clone());
                col_attendance.push(none_value.clone());
                col_wbs.push(none_value.clone());
                col_description.push(none_value.clone());
            }
        }
    }

    let table = Table::from([
        ("Date", col_date),
        ("Start", col_start),
        ("Activity", col_name),
        ("Attendance", col_attendance),
        ("WBS", col_wbs),
        ("Description", col_description),
    ]);

    println!(
        "{}",
        table.to_string_with_options(&table::PrintOptions {
            chars: table::CharOptions::rounded(),
            colors: Some(ColorOptions {
                headers: table::AnsiiColor::Blue,
                lines: table::AnsiiColor::None
            })
        })
    );
}

fn open_entry_file(opts: &opt::Edit) -> Result<()> {
    let editor = env::var("EDITOR").unwrap_or(String::from("vi"));
    _ = Command::new(&editor)
        .arg(get_entry_file_path()?)
        .status()
        .map_err(|e| format_err!("Failed to open {editor}: {e}"))?;
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
