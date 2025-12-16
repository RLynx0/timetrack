use std::{collections::HashMap, fmt::Display, rc::Rc};

use chrono::{DateTime, Local, NaiveDate, TimeDelta};

use crate::activity_entry::TrackedActivity;

/// Grouping of activities with
/// - Same wbs
/// - Same description
/// - Same attendance type
/// - Same local date (precise time is irrelevant)
#[derive(Debug, Clone)]
pub struct CollapsedActivity {
    attendance_type: Rc<str>,
    description: Rc<str>,
    duration: TimeDelta,
    start_of_first: DateTime<Local>,
    wbs: Rc<str>,
}
impl CollapsedActivity {
    pub fn attendance(&self) -> &str {
        &self.attendance_type
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn duration(&self) -> TimeDelta {
        self.duration
    }
    pub fn start_time(&self) -> DateTime<Local> {
        self.start_of_first
    }
    pub fn wbs(&self) -> &str {
        &self.wbs
    }
}
impl Display for CollapsedActivity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\t{:.2}\t{}\t{}\t{}",
            self.start_of_first.format("%Y-%m-%d"),
            self.duration.as_seconds_f64() / 3600.0,
            self.attendance_type,
            self.wbs,
            self.description,
        )
    }
}
pub fn collapse_activities(
    activities: &[TrackedActivity],
    end_fallback: DateTime<Local>,
) -> Vec<CollapsedActivity> {
    let mut grouped_activities = HashMap::new();
    let activities: Vec<_> = activities
        .iter()
        .cloned()
        .flat_map(|t| t.split_on_midnight(end_fallback))
        .collect();
    for activity in &activities {
        grouped_activities
            .entry(ActivityGroupKey::from(activity))
            .or_insert_with(|| CollapsedActivity {
                attendance_type: activity.attendance().into(),
                description: activity.description().into(),
                duration: TimeDelta::zero(),
                start_of_first: *activity.start_time(),
                wbs: activity.wbs().into(),
            })
            .duration +=
            activity.end_time().copied().unwrap_or(Local::now()) - activity.start_time();
    }

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
impl<'a> From<&'a TrackedActivity> for ActivityGroupKey<'a> {
    fn from(activity: &'a TrackedActivity) -> Self {
        ActivityGroupKey {
            wbs: activity.wbs(),
            attendance_type: activity.attendance(),
            description: activity.description(),
            date: activity.start_time().date_naive(),
        }
    }
}

pub struct AttendanceRange {
    start: DateTime<Local>,
    end: Option<DateTime<Local>>,
    attendance_type: Rc<str>,
}
impl AttendanceRange {
    pub fn start_time(&self) -> &DateTime<Local> {
        &self.start
    }
    pub fn end_time(&self) -> Option<&DateTime<Local>> {
        self.end.as_ref()
    }
    pub fn attendance(&self) -> &str {
        &self.attendance_type
    }
}
impl Display for AttendanceRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\t{}\t{}",
            self.start,
            self.end.map(|t| t.to_string()).unwrap_or_default(),
            self.attendance_type
        )
    }
}

pub fn get_attendance_ranges(activities: &[TrackedActivity]) -> Vec<AttendanceRange> {
    let mut ranges = Vec::new();
    let mut last_activity: Option<AttendanceRange> = None;
    for activity in activities {
        if let Some(last) = last_activity {
            last_activity = Some(
                if last.end == Some(*activity.start_time())
                    && activity.attendance() == Rc::as_ref(&last.attendance_type)
                {
                    AttendanceRange {
                        end: activity.end_time().copied(),
                        ..last
                    }
                } else {
                    ranges.push(last);
                    AttendanceRange {
                        start: *activity.start_time(),
                        end: activity.end_time().copied(),
                        attendance_type: Rc::from(activity.attendance()),
                    }
                },
            )
        } else {
            last_activity = Some(AttendanceRange {
                start: *activity.start_time(),
                end: activity.end_time().copied(),
                attendance_type: Rc::from(activity.attendance()),
            });
        }
    }
    ranges.extend(last_activity);
    ranges
}
