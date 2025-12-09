#![allow(unused)] // TODO: Remove this when more things are implemented

use std::{
    env, fs,
    io::{self, IsTerminal, Write, stdin, stdout},
    process::{Command, exit},
    rc::Rc,
    str::FromStr,
};

use chrono::{DateTime, Local, TimeDelta};
use clap::Parser;
use color_eyre::eyre::{Result, format_err};
use rev_lines::RawRevLines;

use crate::{
    activity::{Activity, ActivityEntry},
    activity_range::ActivityRange,
    config::Config,
    files::get_entry_file_path,
    opt::Opt,
};

mod activity;
mod activity_range;
mod config;
mod files;
mod format_string;
mod opt;
mod printable;

const IDLE_WBS_SENTINEL: &str = "Idle";
const BUILTIN_ACTIVITY_IDLE: &str = "Idle";

const ANSII_RED: &str = "\u{001b}[31m";
const ANSII_GREEN: &str = "\u{001b}[32m";
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
        opt::TtrCommand::Show(opts) => show_activities(opts),
        opt::TtrCommand::Edit(_) => open_entry_file(),
        opt::TtrCommand::Generate(_) => todo!(),
        opt::TtrCommand::Activity(_) => todo!(),
    }
}

macro_rules! print_list {
    ($($k:expr => $v: expr,)*) => {
        println!("{}", crate::printable::AlignedList::from([
            $(($k, $v)),*
        ]).with_options(crate::printable::ListPrintOptions {
            colors: std::io::stdout().is_terminal().then_some(
                crate::printable::ColorOptions {
                    headers: crate::printable::AnsiiColor::Blue,
                    lines: crate::printable::AnsiiColor::None,
                }
            ),
            ..Default::default()
        }));
    }
}
macro_rules! print_table {
    ($($k:expr => $vs: expr,)*) => {
        println!("{}", crate::printable::Table::from([
            $(($k, $vs)),*
        ]).with_options(crate::printable::TablePrintOptions {
            chars: crate::printable::TableCharOptions::rounded(),
            colors: std::io::stdout().is_terminal().then_some(
                crate::printable::ColorOptions {
                    headers: crate::printable::AnsiiColor::Blue,
                    lines: crate::printable::AnsiiColor::None,
                }
            ),
        }));
    };
}

fn start_activity(start_opts: &opt::Start) -> Result<()> {
    let config = &get_config()?;
    let activity_name: &str = &start_opts.activity;

    let wbs = resolve_wbs(activity_name)?;

    let last_entry = get_last_entry()?;
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
    if start_opts.verbose {
        print_list! {
            "Description" => description,
            "Attendance" => attendance.to_owned(),
            "WBS" => wbs,
            "Date" => timestamp.format("%Y-%m-%d").to_string(),
            "Time" => timestamp.format("%H:%M:%S").to_string(),
        }
    }
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
    let last_entry = get_last_entry()?;
    match last_entry.as_ref() {
        Some(ActivityEntry::Start(last_start)) => {
            let entry = ActivityEntry::new_end();
            write_entry(&entry)?;

            let stopped = last_start.name();
            println!("Stopped tracking {ANSII_RED}'{stopped}'{ANSII_RESET}");
            let timestamp = entry.time_stamp();
            if end_opts.verbose {
                print_list! {
                    "Date" => timestamp.format("%Y-%m-%d"),
                    "Time" => timestamp.format("%H-%M-%S"),
                }
            }
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

fn show_activities(show_opts: &opt::Show) -> Result<()> {
    match &show_opts.last {
        ActivityRange::Count(0) => show_current_entry(show_opts),
        range => show_activity_range(show_opts, range),
    }
}

fn show_current_entry(show_opts: &opt::Show) -> Result<()> {
    let entry = get_last_entry()?;
    match entry {
        None => println!("You have not recorded any data yet"),
        Some(entry) if show_opts.raw => println!("{entry}"),
        Some(ActivityEntry::End(_)) => {
            println!("You are not tracking any activity")
        }
        Some(ActivityEntry::Start(entry)) => {
            println!(
                "Tracking activity {ANSII_GREEN}'{}'{ANSII_RESET}",
                entry.name()
            );

            let delta = Local::now() - entry.time_stamp();
            print_list! {
                "Description" => entry.description(),
                "Attendance" => entry.attendance(),
                "WBS" => entry.wbs(),
                "Tracked for" => &format_time_delta(&delta),
            }
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

fn show_activity_range(show_opts: &opt::Show, quantity: &ActivityRange) -> Result<()> {
    let activities = match quantity {
        ActivityRange::Count(n) => get_last_n_activities(*n as usize)?,
        ActivityRange::Timeframe(tf) => get_activities_since(&tf.back_from(&Local::now()))?,
    };

    if activities.is_empty() {
        println!("You have not recorded any data yet");
    } else if show_opts.raw {
        for activity in activities {
            println!("{activity}");
        }
    } else {
        print_activitiy_table(activities);
    }

    Ok(())
}

fn print_activitiy_table(activities: impl IntoIterator<Item = Activity>) {
    let mut col_date: Vec<Rc<str>> = Vec::new();
    let mut col_start: Vec<Rc<str>> = Vec::new();
    let mut col_end: Vec<Rc<str>> = Vec::new();
    let mut col_hours: Vec<Rc<str>> = Vec::new();
    let mut col_name: Vec<Rc<str>> = Vec::new();
    let mut col_attendance: Vec<Rc<str>> = Vec::new();
    let mut col_wbs: Vec<Rc<str>> = Vec::new();
    let mut col_description: Vec<Rc<str>> = Vec::new();
    let none_value: Rc<str> = "--".into();

    for activity in activities {
        let start = activity.start_time();
        let time_to = activity.end_time().copied().unwrap_or(Local::now());
        let hours = (time_to - start).as_seconds_f64() / 3600.0;

        col_date.push(start.format("%Y-%m-%d").to_string().into());
        col_start.push(start.format("%H:%M:%S").to_string().into());
        col_end.push(match activity.end_time() {
            Some(t) => t.format("%H:%M:%S").to_string().into(),
            None => none_value.clone(),
        });
        col_hours.push(format!("{hours:.2}").into());
        col_name.push(activity.name().into());
        col_attendance.push(activity.attendance().into());
        col_wbs.push(activity.wbs().into());
        col_description.push(match activity.description() {
            "" => none_value.clone(),
            s => s.into(),
        });
    }

    print_table! {
        "Date" => col_date,
        "Start" => col_start,
        "End" => col_end,
        "Hours" => col_hours,
        "Activity" => col_name,
        "Attendance" => col_attendance,
        "WBS" => col_wbs,
        "Description" => col_description,
    }
}

fn open_entry_file() -> Result<()> {
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

fn get_last_entry() -> Result<Option<ActivityEntry>> {
    let path = &files::get_entry_file_path()?;
    if !fs::exists(path)? {
        return Ok(None);
    }
    let file = fs::File::open(path)?;
    match RawRevLines::new(file).next() {
        Some(l) => Ok(Some(entry_from_byte_result(l)?)),
        None => Ok(None),
    }
}

/// Get the last `count` activities in chronological order
fn get_last_n_activities(count: usize) -> Result<Vec<Activity>> {
    let path = &files::get_entry_file_path()?;
    if !fs::exists(path)? {
        return Ok(Vec::new());
    }

    let file = fs::File::open(path)?;
    let mut rev_lines = RawRevLines::new(file);
    let mut activities = Vec::new();
    let mut last_timestamp = None;
    while let Some(line) = rev_lines.next()
        && activities.len() < count
    {
        let entry = entry_from_byte_result(line)?;
        let end_timestamp = last_timestamp.take();
        last_timestamp = Some(*entry.time_stamp());
        if let ActivityEntry::Start(start_entry) = entry {
            activities.push(Activity::new(start_entry, end_timestamp))
        }
    }

    Ok(activities.into_iter().rev().collect())
}

/// Get activities since `start_time` in chronological order
fn get_activities_since(start_time: &DateTime<Local>) -> Result<Vec<Activity>> {
    let mut activities = Vec::new();
    let mut last_entry = None;
    for entry in get_backwards_entries_since(start_time)?.into_iter().rev() {
        if let Some(last) = last_entry {
            activities.push(Activity::new_completed(last, *entry.time_stamp()));
        }
        last_entry = match entry {
            ActivityEntry::Start(activity_start) => Some(activity_start),
            ActivityEntry::End(_) => None,
        };
    }
    if let Some(last) = last_entry {
        activities.push(Activity::new_ongoing(last));
    }
    Ok(activities)
}

/// Fetch entries since `start_time` in reversed order
fn get_backwards_entries_since(start_time: &DateTime<Local>) -> Result<Vec<ActivityEntry>> {
    let path = &files::get_entry_file_path()?;
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
