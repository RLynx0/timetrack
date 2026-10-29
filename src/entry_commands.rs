use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::Command,
    str::FromStr,
};

use chrono::{DateTime, Local};
use color_eyre::{
    Section,
    eyre::{Result, format_err},
    owo_colors::OwoColorize,
};
use owo_colors::Stream;
use rev_lines::RawRevLines;

use crate::{
    activity_commands::get_trackable_activity,
    activity_entry::{ActivityEntry, TrackedActivity},
    cli,
    files::{self, get_activity_file_path, get_entry_file_path, get_main_config_path},
    get_config, print_smart_list,
    trackable::sanitize_description,
};

pub use generate::handle_generate;
pub use show::show_activities;

mod generate;
mod show;

pub fn start_activity(start_opts: &cli::Start) -> Result<()> {
    let config = &get_config()?;
    let activity_name: &str = &start_opts.activity;
    let activity = get_trackable_activity(activity_name)?;
    let wbs = activity.wbs();

    let last_entry = get_last_entry()?;
    let last_attendance = last_entry.as_ref().and_then(|e| e.attendance_type());
    let attendance = start_opts
        .attendance
        .as_deref()
        .or(last_attendance)
        .unwrap_or(&config.default_attendance);
    if !config.attendance_types.contains_key(attendance) {
        return Err(format_err!("attendance type '{attendance}' is not defined"))
            .with_note(|| "edit your config file to add a new attendance type");
    }

    let description = start_opts
        .description
        .as_deref()
        .or(activity.description())
        .map(sanitize_description)
        .unwrap_or_default();

    let entry = ActivityEntry::new_start(activity_name, attendance, wbs, &description);
    write_entry(&entry)?;

    if let Some(ActivityEntry::Start(last_start)) = last_entry.as_ref() {
        let last_name = last_start.name();
        println!(
            "Stopped tracking '{}'",
            last_name.if_supports_color(Stream::Stdout, |n| n.red())
        );
    }
    println!(
        "Started tracking '{}'",
        activity_name.if_supports_color(Stream::Stdout, |n| n.green())
    );

    let timestamp = entry.time_stamp();
    if start_opts.verbose {
        let attendance_str = match config.attendance_types.get(attendance) {
            Some(hint) if !hint.trim().is_empty() => format!("{attendance} ({hint})"),
            _ => attendance.to_string(),
        };
        print_smart_list! {
            "Description" => description,
            "Attendance" => attendance_str,
            "WBS" => wbs.to_string(),
            "Date" => timestamp.format("%Y-%m-%d").to_string(),
            "Time" => timestamp.format("%H:%M:%S").to_string(),
        }
    }
    Ok(())
}

pub fn end_activity(end_opts: &cli::End) -> Result<()> {
    let last_entry = get_last_entry()?;
    match last_entry.as_ref() {
        Some(ActivityEntry::Start(last_start)) => {
            let entry = ActivityEntry::new_end();
            write_entry(&entry)?;

            let stopped = last_start.name();
            println!(
                "Stopped tracking '{}'",
                stopped.if_supports_color(Stream::Stdout, |n| n.red())
            );
            let timestamp = entry.time_stamp();
            if end_opts.verbose {
                print_smart_list! {
                    "Date" => timestamp.format("%Y-%m-%d"),
                    "Time" => timestamp.format("%H-%M-%S"),
                }
            }
            Ok(())
        }
        _ => Err(color_eyre::eyre::format_err!(
            "You are not tracking any activity"
        )),
    }
}

fn write_entry(entry: &ActivityEntry) -> Result<()> {
    let path = files::get_entry_file_path()?;
    if !fs::exists(&path)?
        && let Some(p) = path.parent()
    {
        fs::create_dir_all(p)?
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    match entry {
        ActivityEntry::Start(_) => writeln!(&mut file, "{entry}")?,
        ActivityEntry::End(_) => writeln!(&mut file, "{entry}\n")?,
    }
    Ok(())
}

pub fn handle_edit(edit_opts: &cli::Edit) -> Result<()> {
    let path = match edit_opts.target {
        cli::EditTarget::Entries => get_entry_file_path(),
        cli::EditTarget::Config => get_main_config_path(),
        cli::EditTarget::Activities => get_activity_file_path(),
    }?;
    open_editor_to_file(&edit_opts.editor, &path)
}

/// Opens the activity log with `EDITOR`
fn open_editor_to_file(editor: &str, path: &Path) -> Result<()> {
    _ = Command::new(editor)
        .arg(path)
        .status()
        .map_err(|e| format_err!("Failed to open {editor}: {e}"))?;
    Ok(())
}

/// Fetch the last recorded activity entry
fn get_last_entry() -> Result<Option<ActivityEntry>> {
    let path = &files::get_entry_file_path()?;
    if !fs::exists(path)? {
        return Ok(None);
    }

    let file = fs::File::open(path)?;
    for byte_res in RawRevLines::new(file) {
        match entry_from_byte_result(byte_res)? {
            Some(entry) => return Ok(Some(entry)),
            None => continue,
        }
    }

    Ok(None)
}

fn get_last_activities_until<T>(
    mut select: impl FnMut(&[TrackedActivity], bool) -> Option<T>,
) -> Result<T> {
    let mut count = 32;
    loop {
        let activities = get_last_n_activities(count)?;
        let exhausted = activities
            .iter()
            .filter(|activity| activity.end_time().is_some())
            .count()
            < count;
        if let Some(selected) = select(&activities, exhausted) {
            return Ok(selected);
        }
        count = count
            .checked_mul(2)
            .ok_or_else(|| format_err!("Too many activities"))?;
    }
}

fn get_activities_between(
    start: Option<DateTime<Local>>,
    end: DateTime<Local>,
    now: DateTime<Local>,
) -> Result<Vec<TrackedActivity>> {
    let path = files::get_entry_file_path()?;
    if !fs::exists(&path)? {
        return Ok(Vec::new());
    }
    let mut activities = Vec::new();
    let mut next_timestamp = None;
    for line in RawRevLines::new(fs::File::open(path)?) {
        let Some(entry) = entry_from_byte_result(line)? else {
            continue;
        };
        let timestamp = *entry.time_stamp();
        if let ActivityEntry::Start(activity) = entry
            && let Some(clipped) =
                TrackedActivity::new(activity, next_timestamp).clipped(start, end, now)
        {
            activities.extend(
                clipped
                    .split_on_midnight(end)
                    .filter(|a| a.end_time().is_none_or(|to| a.start_time() < to))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev(),
            );
        }
        next_timestamp = Some(timestamp);
        if start.is_some_and(|start| timestamp <= start) {
            break;
        }
    }
    activities.reverse();
    Ok(activities)
}

/// Get the last `count` completed pieces plus the ongoing piece, in chronological order.
/// Activities crossing midnight are split, except that zero selects the whole ongoing activity.
fn get_last_n_activities(count: usize) -> Result<Vec<TrackedActivity>> {
    let path = &files::get_entry_file_path()?;
    if !fs::exists(path)? {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path)?;
    let rev_lines = RawRevLines::new(file);
    let now = Local::now();
    let mut activities = Vec::new();
    let mut completed_count = 0;
    let mut last_timestamp = None;
    'entries: for line in rev_lines {
        let Some(entry) = entry_from_byte_result(line)? else {
            continue;
        };
        let end_timestamp = last_timestamp.take();
        last_timestamp = Some(*entry.time_stamp());
        if let ActivityEntry::Start(start) = entry {
            let activity = TrackedActivity::new(start, end_timestamp);
            if count == 0 {
                if activity.end_time().is_none() {
                    activities.push(activity);
                }
                break;
            }
            for activity in activity
                .split_on_midnight(now)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                let completed = activity.end_time().is_some();
                if completed {
                    completed_count += 1;
                }
                activities.push(activity);
                if completed && completed_count == count {
                    break 'entries;
                }
            }
        }
    }

    Ok(activities.into_iter().rev().collect())
}

fn entry_from_byte_result(
    byte_result: std::result::Result<Vec<u8>, io::Error>,
) -> Result<Option<ActivityEntry>> {
    let entry_str = String::from_utf8(byte_result?)?;
    if entry_str.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(ActivityEntry::from_str(&entry_str)?))
    }
}
