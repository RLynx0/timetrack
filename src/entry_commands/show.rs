use std::rc::Rc;

use chrono::{Local, TimeDelta};
use color_eyre::eyre::Result;
use owo_colors::{OwoColorize, Stream};

use crate::{
    NONE_PRINT_VALUE,
    activity_entry::{ActivityEntry, TrackedActivity},
    activity_range::ActivityRange,
    cli,
    entry_commands::generate::CollapsedActivity,
    get_config, print_smart_list, print_smart_table,
};

use super::{
    generate::collapse_activities, get_activities_since, get_last_entry, get_last_n_activities,
};

pub fn show_activities(show_opts: &cli::Show) -> Result<()> {
    match &show_opts.last {
        ActivityRange::Count(0) => show_current_entry(show_opts),
        range => show_activity_range(show_opts, range),
    }
}

fn show_current_entry(show_opts: &cli::Show) -> Result<()> {
    let entry = get_last_entry()?;
    match entry {
        None => println!("You have not recorded any data yet"),
        Some(entry) if show_opts.machine_readable => println!("{entry}"),
        Some(ActivityEntry::End(_)) => {
            println!("You are not tracking any activity")
        }
        Some(ActivityEntry::Start(entry)) => {
            println!(
                "Tracking activity '{}'",
                entry
                    .name()
                    .if_supports_color(Stream::Stdout, |n| n.green())
            );

            let config = get_config()?;
            let delta = Local::now() - entry.time_stamp();
            let attendance = entry.attendance();
            let attendance_str = match config.attendance_types.get(attendance) {
                Some(hint) if !hint.trim().is_empty() => format!("{attendance} ({hint})"),
                _ => attendance.to_string(),
            };
            print_smart_list! {
                "Description" => entry.description(),
                "Attendance" => &attendance_str,
                "WBS" => entry.wbs(),
                "Tracked for" => &format_time_delta(&delta),
            }
        }
    }
    Ok(())
}

fn format_time_delta(delta: &TimeDelta) -> String {
    let mut out = String::new();
    let days = delta.num_days();
    if days > 0 {
        out.push_str(&format!("{days}d "))
    }

    let rem = *delta - TimeDelta::days(days);
    let hours = rem.num_hours();
    if hours > 0 {
        out.push_str(&format!("{hours}h "))
    }

    let rem = rem - TimeDelta::hours(hours);
    let minutes = rem.num_minutes();
    if minutes > 0 {
        out.push_str(&format!("{minutes}m "))
    }

    let rem = rem - TimeDelta::minutes(minutes);
    let seconds = rem.num_seconds();
    out.push_str(&format!("{seconds}s"));

    out
}

fn show_activity_range(show_opts: &cli::Show, quantity: &ActivityRange) -> Result<()> {
    let activities = match quantity {
        ActivityRange::Count(n) => get_last_n_activities(*n as usize)?,
        ActivityRange::Timeframe(tf) => get_activities_since(&tf.back_from(&Local::now()))?,
    };

    if activities.is_empty() {
        if get_last_entry()?.is_none() {
            println!("You have not recorded any data yet")
        } else {
            println!("You have not recorded any data in the requested timeframe");
        }
        return Ok(());
    }

    match show_opts.mode {
        cli::ShowMode::Entries => {
            show_individual_activities(activities, show_opts.machine_readable)
        }
        cli::ShowMode::Collapsed => {
            show_collapsed_activities(&activities, show_opts.machine_readable)
        }
        cli::ShowMode::Time => todo!(),
    }

    Ok(())
}

fn show_individual_activities(
    activities: impl IntoIterator<Item = TrackedActivity>,
    machine_readable: bool,
) {
    if machine_readable {
        for activity in activities {
            println!("{activity}");
        }
    } else {
        print_activitiy_table(activities);
    }
}

fn print_activitiy_table(activities: impl IntoIterator<Item = TrackedActivity>) {
    let mut col_date: Vec<Rc<str>> = Vec::new();
    let mut col_start: Vec<Rc<str>> = Vec::new();
    let mut col_end: Vec<Rc<str>> = Vec::new();
    let mut col_hours: Vec<Rc<str>> = Vec::new();
    let mut col_name: Vec<Rc<str>> = Vec::new();
    let mut col_attendance: Vec<Rc<str>> = Vec::new();
    let mut col_wbs: Vec<Rc<str>> = Vec::new();
    let mut col_description: Vec<Rc<str>> = Vec::new();
    let none_value: Rc<str> = Rc::from(NONE_PRINT_VALUE);

    for activity in activities {
        let start = activity.start_time();
        let time_to = activity.end_time().copied().unwrap_or(Local::now());
        let hours = (time_to - start).as_seconds_f64() / 3600.0;

        col_date.push(start.format("%Y-%m-%d").to_string().into());
        col_start.push(start.format("%H:%M:%S").to_string().into());
        col_end.push(match activity.end_time() {
            Some(t) => t.format("%H:%M:%S").to_string().into(),
            None => none_value.clone(),
        });
        col_hours.push(format!("{hours:.2}").into());
        col_name.push(activity.name().into());
        col_attendance.push(activity.attendance().into());
        col_wbs.push(activity.wbs().into());
        col_description.push(match activity.description() {
            "" => none_value.clone(),
            s => s.into(),
        });
    }

    print_smart_table! {
        "Date" => col_date,
        "Start" => col_start,
        "End" => col_end,
        "Hours" => col_hours,
        "Activity" => col_name,
        "Attendance" => col_attendance,
        "WBS" => col_wbs,
        "Description" => col_description,
    }
}

fn show_collapsed_activities(activities: &[TrackedActivity], machine_readable: bool) {
    let collapsed_activities = collapse_activities(activities, Local::now());
    if machine_readable {
        for collapsed in collapsed_activities {
            println!("{collapsed}");
        }
    } else {
        print_collapsed_activity_table(&collapsed_activities)
    }
}

fn print_collapsed_activity_table(collapsed_activities: &[CollapsedActivity]) {
    let mut col_date: Vec<Rc<str>> = Vec::new();
    let mut col_hours: Vec<Rc<str>> = Vec::new();
    let mut col_attendance: Vec<Rc<str>> = Vec::new();
    let mut col_wbs: Vec<Rc<str>> = Vec::new();
    let mut col_description: Vec<Rc<str>> = Vec::new();
    let none_value: Rc<str> = Rc::from(NONE_PRINT_VALUE);

    for collapsed in collapsed_activities {
        let start = collapsed.start_time();
        let hours = collapsed.duration().as_seconds_f64() / 3600.0;
        col_date.push(start.format("%Y-%m-%d").to_string().into());
        col_hours.push(format!("{hours:.2}").into());
        col_attendance.push(collapsed.attendance().into());
        col_wbs.push(collapsed.wbs().into());
        col_description.push(match collapsed.description() {
            "" => none_value.clone(),
            s => s.into(),
        });
    }

    print_smart_table! {
        "Date" => col_date,
        "Hours" => col_hours,
        "Attendance" => col_attendance,
        "WBS" => col_wbs,
        "Description" => col_description,
    }
}
