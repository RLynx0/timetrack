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
    activity_entry::{ActivityEntry, ActivityStart, TrackedActivity},
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

fn get_last_activity() -> Result<Option<TrackedActivity>> {
    let path = files::get_entry_file_path()?;
    if !fs::exists(&path)? {
        return Ok(None);
    }
    let mut end = None;
    for line in RawRevLines::new(fs::File::open(path)?) {
        if let Some(entry) = entry_from_byte_result(line)? {
            let timestamp = *entry.time_stamp();
            if let ActivityEntry::Start(start) = entry {
                return Ok(Some(TrackedActivity::new(start, end)));
            }
            end = Some(timestamp);
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
        let exhausted = activities.len() < count;
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

/// Get the last `count` activities in chronological order
/// Activities crossing over midnight will be automatically split
fn get_last_n_activities(count: usize) -> Result<Vec<TrackedActivity>> {
    let path = &files::get_entry_file_path()?;
    if !fs::exists(path)? {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path)?;
    let mut rev_lines = RawRevLines::new(file);
    let mut activities = Vec::new();
    let mut last_timestamp = None;
    while let Some(line) = rev_lines.next()
        && activities.len() < count
    {
        let Some(entry) = entry_from_byte_result(line)? else {
            continue;
        };
        let end_timestamp = last_timestamp.take();
        last_timestamp = Some(*entry.time_stamp());
        if let ActivityEntry::Start(start_entry) = entry {
            activities.extend(
                TrackedActivity::new(start_entry, end_timestamp)
                    .split_on_midnight(Local::now())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .take(count - activities.len()),
            );
        }
    }

    Ok(activities.into_iter().rev().collect())
}

/// Get the last `count` completed activity pieces plus the currently ongoing piece.
fn get_last_n_completed_activities_plus_ongoing(count: usize) -> Result<Vec<TrackedActivity>> {
    let path = &files::get_entry_file_path()?;
    if !fs::exists(path)? {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path)?;
    let mut rev_lines = RawRevLines::new(file);
    let mut activities = Vec::new();
    let mut completed_count = 0;
    let mut last_timestamp = None;
    while completed_count < count
        && let Some(line) = rev_lines.next()
    {
        let Some(entry) = entry_from_byte_result(line)? else {
            continue;
        };
        let end_timestamp = last_timestamp.take();
        last_timestamp = Some(*entry.time_stamp());
        if let ActivityEntry::Start(start) = entry {
            for activity in TrackedActivity::new(start, end_timestamp)
                .split_on_midnight(Local::now())
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                if activity.end_time().is_none() {
                    activities.push(activity);
                } else if completed_count < count {
                    activities.push(activity);
                    completed_count += 1;
                }
                if completed_count == count {
                    break;
                }
            }
        }
    }

    Ok(activities.into_iter().rev().collect())
}

/// Get activities since `start_time` in chronological order
/// Activities crossing over midnight will be automatically split
fn get_activities_since(start_time: &DateTime<Local>) -> Result<Vec<TrackedActivity>> {
    let mut activities = Vec::new();
    let mut last_activity_start: Option<ActivityStart> = None;
    for entry in get_backwards_entries_since(start_time)?.into_iter().rev() {
        if let Some(last) = last_activity_start {
            activities.extend(
                TrackedActivity::new_completed(last, *entry.time_stamp())
                    .split_on_midnight(Local::now())
                    .filter(|a| a.end_time().map(|t| t >= start_time).unwrap_or(true)),
            );
        }
        last_activity_start = match entry {
            ActivityEntry::Start(activity_start) => Some(activity_start),
            ActivityEntry::End(_) => None,
        };
    }
    if let Some(last) = last_activity_start {
        activities.push(TrackedActivity::new_ongoing(last));
    }
    Ok(activities)
}

/// Fetch entries since `start_time` in reversed order
/// The first entry before `start_time` will also be included
/// This allows showing an activity that was currently running at `start_time`
fn get_backwards_entries_since(start_time: &DateTime<Local>) -> Result<Vec<ActivityEntry>> {
    let path = &files::get_entry_file_path()?;
    if !fs::exists(path)? {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    let file = fs::File::open(path)?;
    for line in RawRevLines::new(file) {
        let Some(entry) = entry_from_byte_result(line)? else {
            continue;
        };
        let time = *entry.time_stamp();
        entries.push(entry);
        if &time <= start_time {
            break;
        }
    }
    Ok(entries)
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
