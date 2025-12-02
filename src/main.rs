#![allow(unused)] // TODO: Remove this when more things are implemented

const END_SENTINEL: &str = "__END";

use std::{
    collections::HashMap,
    env,
    fmt::Display,
    fs,
    io::{self, BufReader, BufWriter, Read, Seek},
    path::{Path, PathBuf},
    process::exit,
    rc::Rc,
    str::FromStr,
};

use anyhow::anyhow;
use chrono::{DateTime, Datelike, Local, NaiveDate, TimeDelta};
use clap::Parser;
use nom::IResult;
use rev_lines::RawRevLines;

use crate::{config::Config, opt::Opt};

mod config;
mod format_string;
mod opt;

fn get_xdg_config_home() -> anyhow::Result<PathBuf> {
    match env::var("XDG_CONFIG_HOME") {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(_) => Ok(PathBuf::from_iter([
            env::var("HOME")?,
            String::from(".config"),
        ])),
    }
}

fn get_xdg_data_home() -> anyhow::Result<PathBuf> {
    match env::var("XDG_DATA_HOME") {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(_) => Ok(PathBuf::from_iter([
            env::var("HOME")?,
            String::from(".local"),
            String::from("share"),
        ])),
    }
}

fn default_config_path() -> anyhow::Result<PathBuf> {
    let mut config_home = get_xdg_config_home()?;
    config_home.push("timetracker");
    config_home.push("config.toml");
    Ok(config_home)
}

fn get_last_state_entry(path: &Path) -> anyhow::Result<Option<ActivityEntry>> {
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

fn main() {
    let opt = Opt::parse();
    if let opt::SubCommand::DumpDefaultConfig = opt.command {
        println!("{}", include_str!("../default_config.toml"));
        exit(0)
    }

    println!(
        "{:#?}",
        get_last_state_entry(&PathBuf::from("./state_sample"))
    );

    let config_path = opt.config.unwrap_or_else(|| default_config_path().unwrap());
    let config_str = match fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "Failed to read config: {e}\n\n\
                Make sure {config_path:?} exists before running the program!\n\
                You can generate a reference config with the dump-default-config option.\n"
            );
            if let Some(conf_dir) = config_path.parent() {
                eprintln!("  $ mkdir -p {conf_dir:?}");
            }
            eprintln!("  $ timetracker dump-default-config > {config_path:?}");
            exit(1)
        }
    };

    // let config_result = toml::from_str::<Config>(&config_str);
    // println!("{config_result:#?}");
}

#[derive(Debug, Clone)]
enum ParseEntryErr {
    MissingTime,
    MissingName,
    MissingAttendance,
    MissingWbs,
    ParseDatetime(chrono::format::ParseError),
}
impl std::error::Error for ParseEntryErr {}
impl Display for ParseEntryErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseEntryErr::MissingTime => write!(f, "missing time stamp"),
            ParseEntryErr::MissingName => write!(f, "missing activitiy name"),
            ParseEntryErr::MissingAttendance => write!(f, "missing attendance type"),
            ParseEntryErr::MissingWbs => write!(f, "missing wbs"),
            ParseEntryErr::ParseDatetime(parse_error) => {
                write!(f, "failed to parse time stamp: {}", parse_error)
            }
        }
    }
}
impl From<chrono::format::ParseError> for ParseEntryErr {
    fn from(value: chrono::format::ParseError) -> Self {
        ParseEntryErr::ParseDatetime(value)
    }
}

#[derive(Debug, Clone)]
enum ActivityEntry {
    Start(ActivityStart),
    End(DateTime<Local>),
}
impl ActivityEntry {
    fn time_stamp(&self) -> &DateTime<Local> {
        match self {
            ActivityEntry::Start(activity_start) => &activity_start.start,
            ActivityEntry::End(end_time) => end_time,
        }
    }
}
impl FromStr for ActivityEntry {
    type Err = ParseEntryErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut fields = s.split('\t');
        let time_stamp = fields.next().ok_or(ParseEntryErr::MissingTime)?;
        let activity_name = fields.next().ok_or(ParseEntryErr::MissingName)?;

        let time_stamp = DateTime::from_str(time_stamp)?;
        if activity_name == END_SENTINEL {
            return Ok(ActivityEntry::End(time_stamp));
        }

        let attendance_type = fields.next().ok_or(ParseEntryErr::MissingAttendance)?;
        let wbs = fields.next().ok_or(ParseEntryErr::MissingWbs)?;
        let description = fields.next().unwrap_or_default();

        Ok(ActivityEntry::Start(ActivityStart {
            start: time_stamp,
            activity_name: Rc::from(activity_name),
            attendance_type: Rc::from(attendance_type),
            description: Rc::from(description),
            wbs: Rc::from(wbs),
        }))
    }
}

#[derive(Debug, Clone)]
struct ActivityStart {
    start: DateTime<Local>,
    activity_name: Rc<str>,
    attendance_type: Rc<str>,
    description: Rc<str>,
    wbs: Rc<str>,
}

/// Grouping of activities with
/// - Same wbs
/// - Same description
/// - Same attendance type
/// - Same local date (precise time is irrelevant)
#[derive(Debug, Clone)]
struct CollapsedActivity {
    attendance_type: Rc<str>,
    description: Rc<str>,
    duration: TimeDelta,
    start_of_first: DateTime<Local>,
    wbs: Rc<str>,
}

fn group_activities(
    entries: &[ActivityEntry],
    from: &DateTime<Local>,
    to: &DateTime<Local>,
) -> Vec<CollapsedActivity> {
    let mut grouped_activities = HashMap::new();
    let mut previous_entry: Option<&ActivityStart> = None;
    for current_entry in entries
        .iter()
        .skip_while(|e| e.time_stamp() < from)
        .take_while(|e| e.time_stamp() < to)
    {
        if let Some(previous) = previous_entry {
            grouped_activities
                .entry(get_group_key(previous))
                .or_insert_with(|| CollapsedActivity {
                    attendance_type: previous.attendance_type.clone(),
                    description: previous.description.clone(),
                    duration: TimeDelta::zero(),
                    start_of_first: previous.start,
                    wbs: previous.wbs.clone(),
                })
                .duration += *current_entry.time_stamp() - previous.start
        }
        previous_entry = match current_entry {
            ActivityEntry::Start(activity_start) => Some(activity_start),
            ActivityEntry::End(_) => None,
        };
    }

    // TODO: Handle last entry

    let mut grouped_activities = Vec::from_iter(grouped_activities.into_values());
    grouped_activities.sort_unstable_by(|a, b| a.start_of_first.cmp(&b.start_of_first));
    grouped_activities
}

#[derive(PartialEq, Eq, Hash)]
struct ActivityGroupKey<'a> {
    wbs: &'a str,
    attendance_type: &'a str,
    description: &'a str,
    date: NaiveDate,
}
fn get_group_key<'a>(activity: &'a ActivityStart) -> ActivityGroupKey<'a> {
    ActivityGroupKey {
        wbs: &activity.wbs,
        attendance_type: &activity.attendance_type,
        description: &activity.description,
        date: activity.start.naive_local().date(),
    }
}

fn vars_from_config(cfg: &Config) -> HashMap<&'static str, &str> {
    HashMap::from([
        ("employee_name", cfg.employee_name.as_str()),
        ("employee_number", cfg.employee_number.as_str()),
        ("cost_center", cfg.cost_center.as_str()),
        ("performance_type", cfg.performance_type.as_str()),
        ("accounting_cycle", cfg.accounting_cycle.as_str()),
    ])
}

fn vars_from_collapsed_activity(activity: &CollapsedActivity) -> HashMap<&'static str, Rc<str>> {
    let date = activity.start_of_first.naive_local().date();
    let seconds = activity.duration.as_seconds_f64();
    HashMap::from([
        // Regarding date
        ("year", Rc::from(date.year().to_string())),
        ("month", Rc::from(date.month().to_string())),
        ("day", Rc::from(date.year().to_string())),
        // Regarding duration
        ("hours", Rc::from((seconds / 3600.0).to_string())),
        ("minutes", Rc::from((seconds / 60.0).to_string())),
        ("seconds", Rc::from(seconds.to_string())),
        // Other
        ("attendance_type", activity.attendance_type.clone()),
        ("description", activity.description.clone()),
        ("wbs", activity.wbs.clone()),
    ])
}
