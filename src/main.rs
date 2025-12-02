use std::{collections::HashMap, env, fs, path::PathBuf, rc::Rc};

use chrono::{DateTime, Local, NaiveDate, TimeDelta};
use clap::Parser;

use crate::{config::Config, opt::Opt};

mod config;
mod format_string;
mod opt;

fn get_xdg_config_home() -> anyhow::Result<PathBuf> {
    match std::env::var("XDG_CONFIG_HOME") {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(_) => Ok(PathBuf::from_iter([
            std::env::var("HOME")?,
            String::from(".config"),
        ])),
    }
}

fn get_xdg_data_home() -> anyhow::Result<PathBuf> {
    match std::env::var("XDG_DATA_HOME") {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(_) => Ok(PathBuf::from_iter([
            std::env::var("HOME")?,
            String::from(".local"),
            String::from("share"),
        ])),
    }
}

fn get_default_config_path() -> anyhow::Result<PathBuf> {
    let mut config_home = get_xdg_config_home()?;
    config_home.push("timetracker");
    config_home.push("config.toml");
    Ok(config_home)
}

fn main() {
    let opt = Opt::parse();
    println!("{opt:#?}");

    let config_path = get_default_config_path().unwrap();
    if let Some(dir_path) = config_path.parent() {
        fs::create_dir_all(dir_path).unwrap();
    }

    let config_result = toml::from_str::<Config>(include_str!("../default_config.toml"));
    println!("{config_result:#?}");
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

#[derive(Debug, Clone)]
struct ActivityStart {
    attendance_type: Rc<str>,
    description: Rc<str>,
    start: DateTime<Local>,
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
