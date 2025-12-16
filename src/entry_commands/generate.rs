use std::{collections::HashMap, fs, io::Write, path::PathBuf, rc::Rc};

use chrono::{Datelike, Local, NaiveDate};
use color_eyre::eyre::{Result, format_err};

use crate::{
    activity_entry::activity_groupings::{CollapsedActivity, collapse_activities},
    activity_range::InLast,
    cli,
    config::Config,
    get_config,
};

use super::get_activities_since;

pub fn handle_generate(generate_opts: &cli::Generate) -> Result<()> {
    let now = Local::now();
    let start_time = InLast::Months(0).back_from(&now);
    let activities = get_activities_since(&start_time)?;
    let collapsed = collapse_activities(&activities, now);

    let config = get_config()?;
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
        return Ok(());
    }

    let file_vars = vars_per_generated_file(&config, start_time.date_naive());
    let default_name = config.output.file_name_format.evaluate(&file_vars)?;
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
