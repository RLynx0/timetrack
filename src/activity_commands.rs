use std::{
    fs,
    io::Write,
    path::Path,
    rc::Rc,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use color_eyre::{
    Section,
    eyre::{Context, Result, format_err},
    owo_colors::OwoColorize,
};
use owo_colors::Stream;

use crate::{
    NONE_PRINT_VALUE, cli, files, print_smart_list, print_smart_table,
    trackable::{
        Activity, ActivityCategory, ActivityItemRef, ActivityLeaf, BUILTIN_ACTIVITY_IDLE_NAME,
        LookupError, PrintableActivityItem,
    },
};

#[cfg(test)]
mod tests;

pub fn set_activity(set_opts: &cli::SetActivity) -> Result<()> {
    let activity = Activity::new(
        &split_path(&set_opts.name),
        &set_opts.wbs,
        set_opts.description.as_deref(),
    )?;
    let name = activity.full_path();
    let (activities, outcome) = apply_set(get_stored_activities()?, activity, set_opts.force)?;
    write_stored_activities(&activities)?;
    let action = match outcome {
        SetOutcome::Added => "Added",
        SetOutcome::Updated => "Updated",
    };
    println!(
        "{action} activity '{}'",
        name.if_supports_color(Stream::Stdout, |n| n.green())
    );
    Ok(())
}

pub fn move_activity(move_opts: &cli::MoveActivity) -> Result<()> {
    let from = split_path(&move_opts.from);
    let to = split_path(&move_opts.to);
    let (activities, destination) = apply_move(get_stored_activities()?, &from, &to)?;
    write_stored_activities(&activities)?;
    println!(
        "Moved '{}' to '{}'",
        from.join("/")
            .if_supports_color(Stream::Stdout, |n| n.red()),
        destination.if_supports_color(Stream::Stdout, |n| n.green())
    );
    Ok(())
}

pub fn remove_activity(remove_opts: &cli::RemoveActivity) -> Result<()> {
    let path = split_path(&remove_opts.name);
    let (activities, count) = apply_remove(get_stored_activities()?, &path, remove_opts.recursive)?;
    write_stored_activities(&activities)?;
    let noun = if count == 1 { "activity" } else { "activities" };
    println!(
        "Removed {count} {noun} at '{}'",
        path.join("/")
            .if_supports_color(Stream::Stdout, |n| n.red())
    );
    Ok(())
}

fn split_path(path: &str) -> Vec<&str> {
    path.split('/').filter(|part| !part.is_empty()).collect()
}

fn check_mutable_path(path: &[&str]) -> Result<()> {
    if path.is_empty() {
        return Err(format_err!("activity path must not be empty"));
    }
    if path[0] == BUILTIN_ACTIVITY_IDLE_NAME {
        return Err(format_err!(
            "the builtin 'idle' activity cannot be modified"
        ));
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
enum SetOutcome {
    Added,
    Updated,
}

fn apply_set(
    mut activities: Vec<Activity>,
    activity: Activity,
    force: bool,
) -> Result<(Vec<Activity>, SetOutcome)> {
    let path = activity.path_segments();
    check_mutable_path(&path)?;
    let hierarchy =
        ActivityCategory::from(activities.iter().cloned().chain([Activity::builtin_idle()]));
    let outcome = match hierarchy.get_item_at(&path) {
        Ok(ActivityItemRef::Category(_)) => {
            return Err(format_err!("{} is an activity category", path.join("/")));
        }
        Ok(ActivityItemRef::Leaf(_)) if !force => {
            return Err(format_err!("activity '{}' already exists", path.join("/")))
                .with_note(|| "use --force to overwrite an existing activity");
        }
        Ok(ActivityItemRef::Leaf(_)) => {
            for existing in &mut activities {
                if existing.path_segments() == path {
                    *existing = activity.clone();
                }
            }
            SetOutcome::Updated
        }
        Err(LookupError::NoSuchItem { .. }) => {
            activities.push(activity);
            SetOutcome::Added
        }
        Err(error) => return Err(error.into()),
    };
    Ok((activities, outcome))
}

fn apply_remove(
    mut activities: Vec<Activity>,
    path: &[&str],
    recursive: bool,
) -> Result<(Vec<Activity>, usize)> {
    check_mutable_path(path)?;
    let hierarchy =
        ActivityCategory::from(activities.iter().cloned().chain([Activity::builtin_idle()]));
    if matches!(hierarchy.get_item_at(path)?, ActivityItemRef::Category(_)) && !recursive {
        return Err(format_err!("{} is an activity category", path.join("/")))
            .with_note(|| "use --recursive to remove a category and all its activities");
    }
    let original_count = activities.len();
    activities.retain(|activity| !activity.path_segments().starts_with(path));
    let removed = original_count - activities.len();
    Ok((activities, removed))
}

fn apply_move(
    activities: Vec<Activity>,
    from: &[&str],
    to: &[&str],
) -> Result<(Vec<Activity>, String)> {
    check_mutable_path(from)?;
    let hierarchy =
        ActivityCategory::from(activities.iter().cloned().chain([Activity::builtin_idle()]));
    hierarchy.get_item_at(from)?;
    let mut destination = to.to_vec();
    match hierarchy.get_item_at(to) {
        Ok(ActivityItemRef::Category(_)) => destination.push(from[from.len() - 1]),
        Ok(ActivityItemRef::Leaf(_)) => {
            return Err(format_err!("activity '{}' already exists", to.join("/")));
        }
        Err(LookupError::NoSuchItem { .. }) => {}
        Err(error) => return Err(error.into()),
    }
    check_mutable_path(&destination)?;
    if destination.starts_with(from) {
        return Err(format_err!(
            "cannot move an activity or category into itself"
        ));
    }
    match hierarchy.get_item_at(&destination) {
        Ok(_) => {
            return Err(format_err!(
                "destination '{}' already exists",
                destination.join("/")
            ));
        }
        Err(LookupError::NoSuchItem { .. }) => {}
        Err(error) => return Err(error.into()),
    }
    let activities = activities
        .into_iter()
        .map(|activity| {
            activity
                .relocated(from, &destination)
                .map(|relocated| relocated.unwrap_or(activity))
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok((activities, destination.join("/")))
}

pub fn list_activities(ls_opts: &cli::ListActivities) -> Result<()> {
    let search_path = ls_opts.name.as_deref().map(split_path).unwrap_or_default();
    let activities = get_all_trackable_activities()?;
    let hierarchy = ActivityCategory::from(activities);
    let list_mode = match (ls_opts.recursive, ls_opts.collapsed) {
        (true, _) => HierarchyListMode::Recursive,
        (_, true) => HierarchyListMode::Collapsed,
        _ => HierarchyListMode::Flat,
    };
    match hierarchy.get_item_at(&search_path)? {
        ActivityItemRef::Leaf(l) => print_single(l, ls_opts.machine_readable),
        ActivityItemRef::Category(c) => print_hierarchy(c, list_mode, ls_opts.machine_readable),
    };

    Ok(())
}

fn print_single(leaf: &ActivityLeaf, machine_readable: bool) {
    if machine_readable {
        println!("{leaf}");
    } else {
        print_smart_list! {
            "Name" => leaf.name(),
            "WBS" => leaf.wbs(),
            "Description" => leaf.description().unwrap_or_default(),
        }
    }
}

enum HierarchyListMode {
    Flat,
    Collapsed,
    Recursive,
}

fn print_hierarchy(
    hierarchy: &ActivityCategory,
    list_mode: HierarchyListMode,
    machine_readable: bool,
) {
    match list_mode {
        HierarchyListMode::Recursive => {
            let expanded = hierarchy.to_activities_sorted();
            let printable = expanded.iter().map(PrintableActivityItem::Activity);
            print_activities(printable, machine_readable);
        }
        HierarchyListMode::Collapsed => {
            let mut branches: Vec<_> = hierarchy.branches.iter().collect();
            let mut leafs: Vec<_> = hierarchy.leafs.values().collect();
            branches.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
            leafs.sort_unstable_by(|a, b| a.name().cmp(b.name()));
            let printable = branches
                .iter()
                .map(|(name, category)| collapsed_category_item(name, category))
                .chain(
                    leafs
                        .iter()
                        .map(|leaf| PrintableActivityItem::ActivityLeaf(leaf)),
                );
            print_activities(printable, machine_readable);
        }
        HierarchyListMode::Flat => {
            let mut branches = Vec::from_iter(hierarchy.branches.keys());
            let mut leafs = Vec::from_iter(hierarchy.leafs.values());
            leafs.sort_unstable_by(|a, b| a.name().cmp(b.name()));
            branches.sort_unstable();
            let printable = branches
                .iter()
                .map(|s| PrintableActivityItem::CategoryName(s))
                .chain(leafs.iter().map(|s| PrintableActivityItem::ActivityLeaf(s)));
            print_activities(printable, machine_readable);
        }
    }
}

fn collapsed_category_item<'a>(
    name: &str,
    category: &'a ActivityCategory,
) -> PrintableActivityItem<'a> {
    let mut path = String::from(name);
    let mut category = category;
    while category.leafs.is_empty() && category.branches.len() == 1 {
        let (name, child) = category.branches.iter().next().unwrap();
        path.push('/');
        path.push_str(name);
        category = child;
    }
    path.push('/');
    if category.branches.is_empty() && category.leafs.len() == 1 {
        let leaf = category.leafs.values().next().unwrap();
        path.push_str(leaf.name());
        PrintableActivityItem::ActivityPath(path.into(), leaf)
    } else {
        PrintableActivityItem::CategoryPath(path.into())
    }
}

fn print_activities<'a, I>(activities: I, print_machine_readable: bool)
where
    I: IntoIterator<Item = PrintableActivityItem<'a>>,
{
    if print_machine_readable {
        for activity in activities {
            println!("{activity}");
        }
    } else {
        print_activity_table(activities);
    }
}
fn print_activity_table<'a, I>(activities: I)
where
    I: IntoIterator<Item = PrintableActivityItem<'a>>,
{
    let mut col_name: Vec<Rc<str>> = Vec::new();
    let mut col_wbs: Vec<Rc<str>> = Vec::new();
    let mut col_descr: Vec<Rc<str>> = Vec::new();
    let none_value: Rc<str> = NONE_PRINT_VALUE.into();

    for activity in activities {
        let description = match activity.description() {
            Some(d) => Rc::from(d),
            None => none_value.clone(),
        };
        let wbs = match activity.wbs() {
            Some(w) => Rc::from(w),
            None => none_value.clone(),
        };
        col_name.push(activity.display_name());
        col_descr.push(description);
        col_wbs.push(wbs);
    }

    print_smart_table! {
        "Name" => col_name,
        "WBS" => col_wbs,
        "Default Description" => col_descr,
    };
}

pub fn get_trackable_activity(activity_path: &str) -> Result<Activity> {
    get_all_trackable_activities()?
        .into_iter()
        .find(|activity| activity.full_path() == activity_path)
        .ok_or(format_err!("{activity_path} does not exist"))
}

pub fn get_all_trackable_activities() -> Result<Vec<Activity>> {
    let mut activities = get_stored_activities()?;
    activities.push(Activity::builtin_idle());
    Ok(activities)
}

fn get_stored_activities() -> Result<Vec<Activity>> {
    let path = files::get_activity_file_path()?;
    if !fs::exists(&path)? {
        return Ok(Vec::new());
    }
    let activities = fs::read_to_string(path)?
        .lines()
        .map(Activity::from_str)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(activities)
}

fn write_stored_activities(activities: &[Activity]) -> Result<()> {
    write_activities_to(&files::get_activity_file_path()?, activities)
}

fn write_activities_to(path: &Path, activities: &[Activity]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temporary_path = path.with_file_name(format!(
        ".activities.{}.{timestamp}.tmp",
        std::process::id()
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)?;
    let result = (|| -> Result<()> {
        match fs::metadata(path) {
            Ok(metadata) => file.set_permissions(metadata.permissions())?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        for activity in activities {
            writeln!(file, "{activity}")?;
        }
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary_path, path)?;
        Ok(())
    })();
    if let Err(error) = result {
        fs::remove_file(&temporary_path)
            .wrap_err_with(|| format!("failed to clean up {temporary_path:?} after: {error:#}"))?;
        return Err(error);
    }
    Ok(())
}
