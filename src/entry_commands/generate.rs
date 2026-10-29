use std::{collections::HashMap, fs, io::Write, path::PathBuf};

use chrono::{DateTime, Datelike, Local, NaiveDate};
use color_eyre::eyre::{Result, WrapErr, format_err};

use crate::{
    activity_entry::activity_groupings::{
        CollapsedActivity, collapse_activities, latest_collapsed,
    },
    activity_range::ActivityRange,
    cli,
    config::Config,
    environment::Environment,
};

use super::{get_activities_between, get_last_activities_until};

pub fn handle_generate(generate_opts: &cli::Generate, env: &Environment) -> Result<()> {
    let now = Local::now();
    let config = Config::load(env)?;
    let (collapsed, start) = resolve_collapsed(generate_opts, &config, env, now)?;
    let output = render_output(&collapsed, &config)?;

    if generate_opts.stdout {
        print!("{output}");
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
        .create_new(true)
        .write(true)
        .open(&file_path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                format_err!("{file_path:?} already exists")
            } else {
                error.into()
            }
        })?;

    file.write_all(output.as_bytes())?;
    println!("Generated {file_path:?}");

    Ok(())
}

fn resolve_collapsed(
    generate_opts: &cli::Generate,
    config: &Config,
    env: &Environment,
    now: DateTime<Local>,
) -> Result<(Vec<CollapsedActivity>, Option<DateTime<Local>>)> {
    let range = match &generate_opts.range {
        Some(range) => range.clone(),
        None => config
            .generate
            .range
            .parse::<ActivityRange>()
            .wrap_err("Invalid generate.range")?,
    };
    match range {
        ActivityRange::Window(window) => {
            let (start, end) = window
                .resolve(now)
                .wrap_err(if generate_opts.range.is_some() {
                    "Invalid --range"
                } else {
                    "Invalid generate.range"
                })?;
            let activities = get_activities_between(start, end, now, env)?;
            Ok((collapse_activities(&activities, now), start))
        }
        ActivityRange::Count(count) => Ok((
            get_last_activities_until(env, |activities, exhausted| {
                latest_collapsed(activities, count, exhausted, now)
            })?,
            None,
        )),
    }
}

fn render_output(collapsed: &[CollapsedActivity], config: &Config) -> Result<String> {
    let keys = config.generate.keys.join(&config.generate.delimiter);
    let rows = collapsed
        .iter()
        .map(|activity| {
            let vars = vars_per_collapsed_activity(activity);
            config
                .generate
                .values
                .iter()
                .map(|value| value.evaluate(&vars))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map(|values| values.join(&config.generate.delimiter))
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut output = format!("{keys}\r\n");
    if !rows.is_empty() {
        output.push_str(&rows.join("\r\n"));
        output.push_str("\r\n");
    }
    Ok(output)
}

fn date_vars(date: NaiveDate) -> HashMap<&'static str, String> {
    HashMap::from([
        ("year", date.year().to_string()),
        ("month", format!("{:02}", date.month())),
        ("day", format!("{:02}", date.day())),
    ])
}

fn vars_per_generated_file(cfg: &Config, date: NaiveDate) -> HashMap<&'static str, String> {
    let mut vars = date_vars(date);
    vars.extend([
        ("employee_name", cfg.employee_name.clone()),
        ("employee_number", cfg.employee_number.clone()),
        ("cost_center", cfg.cost_center.clone()),
        ("performance_type", cfg.performance_type.clone()),
        ("accounting_cycle", cfg.accounting_cycle.clone()),
    ]);
    vars
}

fn vars_per_collapsed_activity(activity: &CollapsedActivity) -> HashMap<&'static str, String> {
    let date = activity.start_time().date_naive();
    let seconds = activity.duration().as_seconds_f64();
    let mut vars = date_vars(date);
    vars.extend([
        ("hours", format!("{:.2}", seconds / 3600.0)),
        ("minutes", format!("{:.2}", seconds / 60.0)),
        ("seconds", format!("{:.2}", seconds)),
        ("attendance_type", activity.attendance().to_string()),
        ("description", activity.description().to_string()),
        ("wbs", activity.wbs().to_string()),
    ]);
    vars
}
