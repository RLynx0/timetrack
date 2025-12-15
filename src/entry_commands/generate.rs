use std::{collections::HashMap, fmt::Display, fs, io::Write, rc::Rc};

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeDelta};
use color_eyre::eyre::Result;

use crate::{
    activity_entry::TrackedActivity, activity_range::InLast, cli, config::Config, get_config,
};

use super::get_activities_since;

pub fn handle_generate(generate_opts: &cli::Generate) -> Result<()> {
    let now = Local::now();
    let start_time = InLast::Months(0).back_from(&now);
    let activities = get_activities_since(&start_time)?;
    let collapsed = collapse_activities(&activities, now);

    let config = get_config()?;
    let file_vars = vars_per_generated_file(&config, start_time.date_naive());
    let file_name = config.output.file_name_format.evaluate(&file_vars)?;
    let file_name = generate_opts.file_path.as_ref().unwrap_or(&file_name);

    let keys = config.output.keys.join(&config.output.delimiter);
    let lines = collapsed
        .iter()
        .map(|c| {
            let vars = vars_per_collapsed_activity(c);
            config
                .output
                .values
                .iter()
                .map(|v| v.evaluate(&vars))
                .collect::<core::result::Result<Vec<_>, _>>()
                .map(|s| s.join(&config.output.delimiter))
        })
        .collect::<core::result::Result<Vec<_>, _>>()?
        .join("\r\n");

    if generate_opts.stdout {
        println!("{keys}\n{lines}");
    } else {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file_name)?;
        writeln!(&mut file, "{keys}\n{lines}")?;
        println!("Generated {file_name}");
    }

    Ok(())
}

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
            .entry(get_group_key(activity))
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

fn get_group_key<'a>(activity: &'a TrackedActivity) -> ActivityGroupKey<'a> {
    ActivityGroupKey {
        wbs: activity.wbs(),
        attendance_type: activity.attendance(),
        description: activity.description(),
        date: activity.start_time().date_naive(),
    }
}

fn vars_per_generated_file(cfg: &Config, date: NaiveDate) -> HashMap<&'static str, Rc<str>> {
    HashMap::from([
        // From config
        ("employee_name", Rc::from(cfg.employee_name.as_str())),
        ("employee_number", Rc::from(cfg.employee_number.as_str())),
        ("cost_center", Rc::from(cfg.cost_center.as_str())),
        ("performance_type", Rc::from(cfg.performance_type.as_str())),
        ("accounting_cycle", Rc::from(cfg.accounting_cycle.as_str())),
        // Regarding date
        ("year", Rc::from(date.year().to_string())),
        ("month", Rc::from(format!("{:02}", date.month()))),
        ("day", Rc::from(format!("{:02}", date.day()))),
    ])
}

fn vars_per_collapsed_activity(activity: &CollapsedActivity) -> HashMap<&'static str, Rc<str>> {
    let date = activity.start_of_first.date_naive();
    let seconds = activity.duration.as_seconds_f64();
    HashMap::from([
        // Regarding date
        ("year", Rc::from(date.year().to_string())),
        ("month", Rc::from(format!("{:02}", date.month()))),
        ("day", Rc::from(format!("{:02}", date.day()))),
        // Regarding duration
        ("hours", Rc::from(format!("{:.2}", seconds / 3600.0))),
        ("minutes", Rc::from(format!("{:.2}", seconds / 60.0))),
        ("seconds", Rc::from(format!("{:.2}", seconds))),
        // Other
        ("attendance_type", activity.attendance_type.clone()),
        ("description", activity.description.clone()),
        ("wbs", activity.wbs.clone()),
    ])
}
