use std::{fmt::Display, rc::Rc, str::FromStr};

use chrono::{DateTime, Local};

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
pub struct Activity {
    pub start_entry: ActivityStart,
    pub end: Option<DateTime<Local>>,
}
impl Activity {
    pub fn new(start_entry: ActivityStart, end: Option<DateTime<Local>>) -> Self {
        Activity { start_entry, end }
    }
    pub fn new_completed(start_entry: ActivityStart, end: DateTime<Local>) -> Self {
        Activity {
            start_entry,
            end: Some(end),
        }
    }
    pub fn new_ongoing(start_entry: ActivityStart) -> Self {
        Activity {
            start_entry,
            end: None,
        }
    }

    pub fn start_time(&self) -> &DateTime<Local> {
        self.start_entry.time_stamp()
    }
    pub fn end_time(&self) -> Option<&DateTime<Local>> {
        self.end.as_ref()
    }
    pub fn name(&self) -> &str {
        self.start_entry.name()
    }
    pub fn attendance(&self) -> &str {
        self.start_entry.attendance()
    }
    pub fn description(&self) -> &str {
        self.start_entry.description()
    }
    pub fn wbs(&self) -> &str {
        self.start_entry.wbs()
    }
}
impl Display for Activity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ActivityStart {
            time_stamp: start,
            activity_name: name,
            attendance_type: attendance,
            description: descr,
            wbs,
        } = &self.start_entry;
        let end = self.end_time().map(|s| s.to_string()).unwrap_or_default();
        write!(f, "{start}\t{end}\t{name}\t{attendance}\t{wbs}\t{descr}")
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
            ActivityEntry::Start(start) => start.time_stamp(),
            ActivityEntry::End(end) => end.time_stamp(),
        }
    }
    pub fn attendance_type(&self) -> Option<&str> {
        match self {
            ActivityEntry::Start(activity_start) => Some(&activity_start.attendance_type),
            ActivityEntry::End(_) => None,
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
impl ActivityEnd {
    pub fn time_stamp(&self) -> &DateTime<Local> {
        &self.time_stamp
    }
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
    pub fn time_stamp(&self) -> &DateTime<Local> {
        &self.time_stamp
    }
    pub fn name(&self) -> &str {
        &self.activity_name
    }
    pub fn attendance(&self) -> &str {
        &self.attendance_type
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn wbs(&self) -> &str {
        &self.wbs
    }
}
