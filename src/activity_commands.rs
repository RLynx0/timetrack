use std::{
    fmt::Display,
    fs,
    io::Write,
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};

use color_eyre::{
    Section,
    eyre::{Result, format_err},
};

use crate::{
    NONE_PRINT_VALUE, cli, files, print_smart_list, print_smart_table,
    trackable::{
        Activity, ActivityCategory, ActivityItemRef, ActivityLeaf, BUILTIN_ACTIVITY_IDLE_NAME,
        PrintableActivityItem,
    },
};

pub fn set_activity(set_opts: &cli::SetActivity) -> Result<()> {
    let activities = get_all_trackable_activities()?;
    let hierarchy = ActivityCategory::from(activities);

    todo!()
}

pub fn move_activity(move_opts: &cli::MoveActivity) -> Result<()> {
    todo!()
}

pub fn remove_activity(set_opts: &cli::RemoveActivity) -> Result<()> {
    todo!()
}

pub fn list_activities(opts: &cli::ListActivities) -> Result<()> {
    let search_path = opts
        .name
        .as_deref()
        .map(|s| s.split("/").filter(|s| !s.is_empty()).collect::<Vec<_>>())
        .unwrap_or_default();
    let activities = get_all_trackable_activities()?;
    let hierarchy = ActivityCategory::from(activities);
    match hierarchy.get_item_at(&search_path).unwrap() {
        ActivityItemRef::Leaf(l) => print_single(l, opts.machine_readable),
        ActivityItemRef::Category(c) => print_hierarchy(c, opts.recursive, opts.machine_readable),
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

fn print_hierarchy(hierarchy: &ActivityCategory, recursive: bool, machine_readable: bool) {
    if recursive {
        let expanded = hierarchy.to_activities_sorted();
        let printable = expanded.iter().map(PrintableActivityItem::Activity);
        print_activities(printable, machine_readable);
    } else {
        let mut branches = Vec::from_iter(hierarchy.branches.keys());
        let mut leafs = Vec::from_iter(hierarchy.leafs.values());
        leafs.sort_unstable_by(|a, b| a.name().cmp(b.name()));
        branches.sort_unstable();
        let printable = branches
            .iter()
            .map(|s| PrintableActivityItem::CategoryName(s))
            .chain(leafs.iter().map(|s| PrintableActivityItem::ActivityLeaf(s)));
        print_activities(printable, machine_readable);
    };
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
    let path = files::get_activity_file_path()?;
    let builtin_idle = Activity::builtin_idle();
    if !fs::exists(&path)? {
        return Ok(vec![builtin_idle]);
    }
    let mut activities = fs::read_to_string(path)?
        .lines()
        .map(Activity::from_str)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    activities.push(builtin_idle);
    Ok(activities)
}
