use std::rc::Rc;

use chrono::{DurationRound, Local, TimeDelta};
use color_eyre::eyre::Result;
use itertools::Itertools;
use owo_colors::{OwoColorize, Stream};

use crate::{
    NONE_PRINT_VALUE,
    activity_entry::{
        ActivityEntry, TrackedActivity,
        activity_groupings::{
            AttendanceRange, CollapsedActivity, collapse_activities, get_attendance_ranges,
            latest_attendance_ranges, latest_collapsed,
        },
    },
    activity_range::ActivityRange,
    cli, get_config, print_smart_list, print_smart_table,
};

use super::{
    get_activities_between, get_last_activities_until, get_last_activity, get_last_entry,
    get_last_n_completed_activities_plus_ongoing,
};

pub fn show_activities(show_opts: &cli::Show) -> Result<()> {
    let now = Local::now();
    let window_activities = match &show_opts.range {
        ActivityRange::Window(window) => {
            let (start, end) = window.resolve(now)?;
            Some(get_activities_between(start, end, now)?)
        }
        ActivityRange::Count(_) => None,
    };

    for (i, mode) in show_opts.mode.iter().unique().enumerate() {
        if i > 0 {
            println!();
        }
        if show_opts.mode.len() > 1 {
            println!(
                ":{}",
                match mode {
                    cli::ShowMode::Entries => "Entries",
                    cli::ShowMode::Collapsed => "Collapsed",
                    cli::ShowMode::Attendance => "Attendance",
                    cli::ShowMode::Time => "Time",
                }
            );
        }
        if let Some(activities) = &window_activities {
            if activities.is_empty() {
                show_empty()?;
                continue;
            }
            match mode {
                cli::ShowMode::Entries => {
                    show_individual_activities(activities, show_opts.machine_readable)?
                }
                cli::ShowMode::Collapsed => {
                    show_collapsed_activities(activities, show_opts.machine_readable)?
                }
                cli::ShowMode::Attendance => {
                    show_daily_attendance(activities, show_opts.machine_readable)?
                }
                cli::ShowMode::Time => show_activity_time(activities, show_opts.machine_readable),
            }
            continue;
        }
        let ActivityRange::Count(count) = &show_opts.range else {
            unreachable!()
        };
        let latest = *count == 0;
        if latest && *mode != cli::ShowMode::Entries && show_inactive(show_opts)? {
            continue;
        }
        match mode {
            cli::ShowMode::Entries if latest => show_current_entry(show_opts)?,
            cli::ShowMode::Entries | cli::ShowMode::Time => {
                let activities = if latest {
                    get_last_activity()?.into_iter().collect()
                } else {
                    get_last_n_completed_activities_plus_ongoing(*count)?
                };
                if activities.is_empty() {
                    show_empty()?;
                } else if *mode == cli::ShowMode::Time {
                    show_activity_time(&activities, show_opts.machine_readable);
                } else {
                    show_individual_activities(&activities, show_opts.machine_readable)?;
                }
            }
            cli::ShowMode::Collapsed => {
                let rows = get_last_activities_until(|activities, exhausted| {
                    latest_collapsed(activities, *count, exhausted, now)
                })?;
                if rows.is_empty() {
                    show_empty()?;
                } else if show_opts.machine_readable {
                    for row in rows {
                        println!("{row}");
                    }
                } else {
                    print_collapsed_activity_table(&rows, latest)?;
                }
            }
            cli::ShowMode::Attendance => {
                let rows = get_last_activities_until(|activities, exhausted| {
                    latest_attendance_ranges(activities, *count, exhausted)
                })?;
                if rows.is_empty() {
                    show_empty()?;
                } else if show_opts.machine_readable {
                    for row in rows {
                        println!("{row}");
                    }
                } else {
                    print_attendance_table(&rows, latest)?;
                }
            }
        }
    }
    Ok(())
}

fn show_inactive(show_opts: &cli::Show) -> Result<bool> {
    match get_last_entry()? {
        None => {
            show_empty()?;
            Ok(true)
        }
        Some(ActivityEntry::End(_)) => {
            if !show_opts.machine_readable {
                println!("You are not tracking any activity");
            }
            Ok(true)
        }
        Some(ActivityEntry::Start(_)) => Ok(false),
    }
}

fn show_empty() -> Result<()> {
    if get_last_entry()?.is_none() {
        println!("You have not recorded any data yet");
    } else {
        println!("You have not recorded any data in the requested timeframe");
    }
    Ok(())
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

// ------- //
// Entries //
// ------- //

fn show_individual_activities(
    activities: &[TrackedActivity],
    machine_readable: bool,
) -> Result<()> {
    if machine_readable {
        for activity in activities {
            println!("{activity}");
        }
    } else {
        print_activitiy_table(activities)?;
    }

    Ok(())
}

fn print_activitiy_table(activities: &[TrackedActivity]) -> Result<()> {
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

        let config = get_config()?;
        let attendance = activity.attendance();
        let attendance_str = match config.attendance_types.get(attendance) {
            Some(hint) if !hint.trim().is_empty() => format!("{attendance} ({hint})"),
            _ => attendance.to_string(),
        };

        col_date.push(start.format("%Y-%m-%d").to_string().into());
        col_start.push(start.format("%H:%M:%S").to_string().into());
        col_end.push(match activity.end_time() {
            Some(t) => t.format("%H:%M:%S").to_string().into(),
            None => none_value.clone(),
        });
        col_hours.push(format!("{hours:.2}").into());
        col_name.push(activity.name().into());
        col_attendance.push(attendance_str.into());
        col_wbs.push(activity.wbs().into());
        col_description.push(match activity.description() {
            "" => none_value.clone(),
            s => s.into(),
        });
    }

    print_smart_table! {
        "Date" => col_date,
        "Start" => col_start,
        "Hours" => col_hours,
        "End" => col_end,
        "Activity" => col_name,
        "WBS" => col_wbs,
        "Attendance" => col_attendance,
        "Description" => col_description,
    }

    Ok(())
}

// --------- //
// Collapsed //
// --------- //

fn show_collapsed_activities(activities: &[TrackedActivity], machine_readable: bool) -> Result<()> {
    let collapsed_activities = collapse_activities(activities, Local::now());
    if machine_readable {
        for collapsed in collapsed_activities {
            println!("{collapsed}");
        }
    } else {
        print_collapsed_activity_table(&collapsed_activities, false)?
    }

    Ok(())
}

fn print_collapsed_activity_table(
    collapsed_activities: &[CollapsedActivity],
    single: bool,
) -> Result<()> {
    let mut col_date: Vec<Rc<str>> = Vec::new();
    let mut col_hours: Vec<Rc<str>> = Vec::new();
    let mut col_attendance: Vec<Rc<str>> = Vec::new();
    let mut col_wbs: Vec<Rc<str>> = Vec::new();
    let mut col_description: Vec<Rc<str>> = Vec::new();
    let none_value: Rc<str> = Rc::from(NONE_PRINT_VALUE);

    for collapsed in collapsed_activities {
        let start = collapsed.start_time();
        let hours = collapsed.duration().as_seconds_f64() / 3600.0;

        let config = get_config()?;
        let attendance = collapsed.attendance();
        let attendance_str = match config.attendance_types.get(attendance) {
            Some(hint) if !hint.trim().is_empty() => format!("{attendance} ({hint})"),
            _ => attendance.to_string(),
        };

        col_date.push(start.format("%Y-%m-%d").to_string().into());
        col_hours.push(format!("{hours:.2}").into());
        col_attendance.push(attendance_str.into());
        col_wbs.push(collapsed.wbs().into());
        col_description.push(match collapsed.description() {
            "" => none_value.clone(),
            s => s.into(),
        });
    }

    if single {
        print_smart_list! {
            "Date" => &col_date[0],
            "Hours" => &col_hours[0],
            "WBS" => &col_wbs[0],
            "Attendance" => &col_attendance[0],
            "Description" => &col_description[0],
        }
        return Ok(());
    }
    print_smart_table! {
        "Date" => col_date,
        "Hours" => col_hours,
        "WBS" => col_wbs,
        "Attendance" => col_attendance,
        "Description" => col_description,
    }

    Ok(())
}

// ---------- //
// Attendance //
// ---------- //

fn show_daily_attendance(activities: &[TrackedActivity], machine_readable: bool) -> Result<()> {
    let ranges = get_attendance_ranges(activities);
    if machine_readable {
        for range in ranges {
            println!("{range}");
        }
    } else {
        print_attendance_table(&ranges, false)?;
    }

    Ok(())
}

fn print_attendance_table(ranges: &[AttendanceRange], single: bool) -> Result<()> {
    let mut col_date: Vec<Rc<str>> = Vec::new();
    let mut col_start: Vec<Rc<str>> = Vec::new();
    let mut col_end: Vec<Rc<str>> = Vec::new();
    let mut col_hours: Vec<Rc<str>> = Vec::new();
    let mut col_hours_adjusted: Vec<Rc<str>> = Vec::new();
    let mut col_attendance: Vec<Rc<str>> = Vec::new();
    let none_value: Rc<str> = NONE_PRINT_VALUE.into();

    for range in ranges {
        let quantum = TimeDelta::minutes(15);
        let start = range.start_time().duration_trunc(quantum).unwrap();
        let end_value = range.end_time().copied().unwrap_or(Local::now());
        let end = end_value.duration_round_up(quantum).unwrap();
        let end_str = range
            .end_time()
            .map(|t| t.duration_round_up(quantum).unwrap())
            .map(|t| t.format("%H:%M").to_string().into())
            .unwrap_or(none_value.clone());

        let delta = end - start;
        let delta_adjusted = match delta {
            d if d <= TimeDelta::hours(6) => d,
            d => d - TimeDelta::minutes(30),
        };

        let hours = delta.as_seconds_f64() / 3600.0;
        let hours_adjusted = delta_adjusted.as_seconds_f64() / 3600.0;

        let config = get_config()?;
        let attendance = range.attendance();
        let attendance_str = match config.attendance_types.get(attendance) {
            Some(hint) if !hint.trim().is_empty() => format!("{attendance} ({hint})"),
            _ => attendance.to_string(),
        };

        col_date.push(start.format("%Y-%m-%d").to_string().into());
        col_start.push(start.format("%H:%M").to_string().into());
        col_end.push(end_str);
        col_hours.push(format!("{hours:.2}").into());
        col_hours_adjusted.push(format!("{hours_adjusted:.2}").into());
        col_attendance.push(attendance_str.into());
    }

    if single {
        print_smart_list! {
            "Date" => &col_date[0],
            "Start" => &col_start[0],
            "End" => &col_end[0],
            "Hours" => &col_hours[0],
            "Adjusted Hours" => &col_hours_adjusted[0],
            "Attendance" => &col_attendance[0],
        }
        return Ok(());
    }
    print_smart_table! {
        "Date" => col_date,
        "Start" => col_start,
        "End" => col_end,
        "Hours" => col_hours,
        "Adjusted Hours" => col_hours_adjusted,
        "Attendance" => col_attendance,
    }

    Ok(())
}

// ---- //
// Time //
// ---- //

fn show_activity_time(activities: &[TrackedActivity], machine_readable: bool) {
    let sum: TimeDelta = activities
        .iter()
        .map(|a| a.end_time().copied().unwrap_or(Local::now()) - a.start_time())
        .sum();
    if machine_readable {
        println!("{:.2}", sum.as_seconds_f64());
    } else {
        print_smart_list! {
            "Tracked Time" => format_time_delta(&sum),
        };
    }
}

// ------- //
// General //
// ------- //

fn format_time_delta(delta: &TimeDelta) -> String {
    let mut out = String::new();
    let days = delta.num_days();
    if days != 0 {
        out.push_str(&format!("{days}d "))
    }

    let rem = *delta - TimeDelta::days(days);
    let hours = rem.num_hours();
    if hours != 0 {
        out.push_str(&format!("{hours}h "))
    }

    let rem = rem - TimeDelta::hours(hours);
    let minutes = rem.num_minutes();
    if minutes != 0 {
        out.push_str(&format!("{minutes}m "))
    }

    let rem = rem - TimeDelta::minutes(minutes);
    let seconds = rem.num_seconds();
    out.push_str(&format!("{seconds}s"));

    out
}
