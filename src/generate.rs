use std::{collections::HashMap, rc::Rc};

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeDelta};

use crate::{
    activity_entry::{ActivityEntry, ActivityStart},
    config::Config,
};

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
                    attendance_type: Rc::from(previous.attendance()),
                    description: Rc::from(previous.description()),
                    duration: TimeDelta::zero(),
                    start_of_first: *previous.time_stamp(),
                    wbs: Rc::from(previous.wbs()),
                })
                .duration += *current_entry.time_stamp() - previous.time_stamp()
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
        wbs: activity.wbs(),
        attendance_type: activity.attendance(),
        description: activity.description(),
        date: activity.time_stamp().naive_local().date(),
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
