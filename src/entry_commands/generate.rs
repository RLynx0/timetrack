use std::{collections::HashMap, fs, io::Write, path::PathBuf, rc::Rc};

use chrono::{Datelike, Local, NaiveDate};
use color_eyre::eyre::{Result, WrapErr, format_err};

use crate::{
    activity_entry::activity_groupings::{
        CollapsedActivity, collapse_activities, latest_collapsed,
    },
    activity_range::ActivityRange,
    cli,
    config::Config,
    get_config,
};

use super::{get_activities_between, get_last_activities_until};

pub fn handle_generate(generate_opts: &cli::Generate) -> Result<()> {
    let now = Local::now();
    let config = get_config()?;
    let range = match &generate_opts.range {
        Some(range) => range.clone(),
        None => config
            .generate
            .range
            .parse::<ActivityRange>()
            .wrap_err("Invalid generate.range")?,
    };
    let (collapsed, start) = match range {
        ActivityRange::Window(window) => {
            let (start, end) = window
                .resolve(now)
                .wrap_err(if generate_opts.range.is_some() {
                    "Invalid --range"
                } else {
                    "Invalid generate.range"
                })?;
            let activities = get_activities_between(start, end, now)?;
            (collapse_activities(&activities, now), start)
        }
        ActivityRange::Count(count) => (
            get_last_activities_until(|activities, exhausted| {
                latest_collapsed(activities, count, exhausted, now)
            })?,
            None,
        ),
    };
    let keys = config.generate.keys.join(&config.generate.delimiter);
    let lines = collapsed
        .iter()
        .map(|c| {
            let vars = vars_per_collapsed_activity(c);
            config
                .generate
                .values
                .iter()
                .map(|v| v.evaluate(&vars))
                .collect::<core::result::Result<Vec<_>, _>>()
                .map(|s| s.join(&config.generate.delimiter))
        })
        .collect::<core::result::Result<Vec<_>, _>>()?
        .join("\r\n");

    if generate_opts.stdout {
        println!("{keys}\n{lines}");
        return Ok(());
    }

    let file_date = start
        .map(|start| start.date_naive())
        .or_else(|| collapsed.first().map(|row| row.start_time().date_naive()))
        .unwrap_or_else(|| now.date_naive());
    let file_vars = vars_per_generated_file(&config, file_date);
    let default_name = config.generate.file_name_format.evaluate(&file_vars)?;
    let file_path = generate_opts.file_path.as_ref().unwrap_or(&default_name);
    let mut file_path = PathBuf::from(file_path);
    while fs::exists(&file_path)? {
        if file_path.is_dir() {
            file_path.push(&default_name);
        } else {
            return Err(format_err!("{file_path:?} already exists"));
        }
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&file_path)?;

    writeln!(&mut file, "{keys}\n{lines}")?;
    println!("Generated {file_path:?}");

    Ok(())
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
    let date = activity.start_time().date_naive();
    let seconds = activity.duration().as_seconds_f64();
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
        ("attendance_type", activity.attendance().into()),
        ("description", activity.description().into()),
        ("wbs", activity.wbs().into()),
    ])
}
