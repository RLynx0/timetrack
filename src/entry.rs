use std::{collections::HashMap, fmt::Display, rc::Rc, str::FromStr};

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeDelta};

use crate::config::Config;

const END_SENTINEL: &str = "__END";

#[derive(Debug, Clone)]
pub enum ParseEntryError {
    MissingTime,
    MissingName,
    MissingAttendance,
    MissingWbs,
    ParseDatetime(chrono::format::ParseError),
}
impl std::error::Error for ParseEntryError {}
impl Display for ParseEntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseEntryError::MissingTime => write!(f, "missing time stamp"),
            ParseEntryError::MissingName => write!(f, "missing activitiy name"),
            ParseEntryError::MissingAttendance => write!(f, "missing attendance type"),
            ParseEntryError::MissingWbs => write!(f, "missing wbs"),
            ParseEntryError::ParseDatetime(parse_error) => {
                write!(f, "failed to parse time stamp: {}", parse_error)
            }
        }
    }
}
impl From<chrono::format::ParseError> for ParseEntryError {
    fn from(value: chrono::format::ParseError) -> Self {
        ParseEntryError::ParseDatetime(value)
    }
}

#[derive(Debug, Clone)]
pub enum ActivityEntry {
    Start(ActivityStart),
    End(ActivityEnd),
}
impl ActivityEntry {
    pub fn new_start(
        activity_name: &str,
        attendance_type: &str,
        wbs: &str,
        description: &str,
    ) -> Self {
        ActivityEntry::Start(ActivityStart {
            time_stamp: Local::now(),
            activity_name: Rc::from(activity_name),
            attendance_type: Rc::from(attendance_type),
            description: Rc::from(description),
            wbs: Rc::from(wbs),
        })
    }
    pub fn new_end() -> Self {
        ActivityEntry::End(ActivityEnd {
            time_stamp: Local::now(),
        })
    }
    pub fn time_stamp(&self) -> &DateTime<Local> {
        match self {
            ActivityEntry::Start(start) => &start.time_stamp,
            ActivityEntry::End(end) => &end.time_stamp,
        }
    }
    pub fn attendance_type(&self) -> Option<&str> {
        match self {
            ActivityEntry::Start(activity_start) => Some(&activity_start.attendance_type),
            ActivityEntry::End(activity_end) => None,
        }
    }
}
impl FromStr for ActivityEntry {
    type Err = ParseEntryError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut fields = s.split('\t');
        let time_stamp = fields.next().ok_or(ParseEntryError::MissingTime)?;
        let activity_name = fields.next().ok_or(ParseEntryError::MissingName)?;

        let time_stamp = DateTime::from_str(time_stamp)?;
        if activity_name == END_SENTINEL {
            return Ok(ActivityEntry::End(ActivityEnd { time_stamp }));
        }

        let attendance_type = fields.next().ok_or(ParseEntryError::MissingAttendance)?;
        let wbs = fields.next().ok_or(ParseEntryError::MissingWbs)?;
        let description = fields.next().unwrap_or_default();

        Ok(ActivityEntry::Start(ActivityStart {
            time_stamp,
            activity_name: Rc::from(activity_name),
            attendance_type: Rc::from(attendance_type),
            description: Rc::from(description),
            wbs: Rc::from(wbs),
        }))
    }
}
impl Display for ActivityEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityEntry::End(ActivityEnd { time_stamp }) => {
                write!(f, "{time_stamp}\t{END_SENTINEL}")
            }
            ActivityEntry::Start(ActivityStart {
                time_stamp,
                activity_name,
                attendance_type,
                description,
                wbs,
            }) => write!(
                f,
                "{time_stamp}\t{activity_name}\t{attendance_type}\t{wbs}\t{description}"
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActivityEnd {
    time_stamp: DateTime<Local>,
}

#[derive(Debug, Clone)]
pub struct ActivityStart {
    time_stamp: DateTime<Local>,
    activity_name: Rc<str>,
    attendance_type: Rc<str>,
    description: Rc<str>,
    wbs: Rc<str>,
}
impl ActivityStart {
    pub fn attendance(&self) -> &str {
        &self.attendance_type
    }
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
                    start_of_first: previous.time_stamp,
                    wbs: previous.wbs.clone(),
                })
                .duration += *current_entry.time_stamp() - previous.time_stamp
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
        date: activity.time_stamp.naive_local().date(),
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
