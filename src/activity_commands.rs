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
    NONE_PRINT_VALUE, files, opt, print_smart_table,
    trackable::{Activity, ActivityCategory, ActivityLeaf, PrintableActivityItem},
};

pub fn set_activity(set_opts: &opt::SetActivity) -> Result<()> {
    todo!()
}

pub fn remove_activity(set_opts: &opt::RemoveActivity) -> Result<()> {
    todo!()
}

pub fn list_activities(opts: &opt::ListActivities) -> Result<()> {
    let mut activities = get_all_trackable_activities()?;
    let hierarchy = ActivityCategory::from(activities);

    let printable: Vec<_> = if opts.expand {
        hierarchy
            .as_activities_sorted()
            .into_iter()
            .map(PrintableActivityItem::Activity)
            .collect()
    } else {
        let mut branch_names = Vec::from_iter(hierarchy.branches.into_keys());
        let mut leafs = Vec::from_iter(hierarchy.leafs.into_values());
        leafs.sort_unstable_by(|a, b| a.name().cmp(b.name()));
        branch_names.sort_unstable();
        branch_names
            .into_iter()
            .map(PrintableActivityItem::CategoryName)
            .chain(leafs.into_iter().map(PrintableActivityItem::ActivityLeaf))
            .collect()
    };

    if opts.raw {
        for activity in printable {
            println!("{activity}");
        }
    } else {
        print_activity_table(printable);
    }

    Ok(())
}

fn print_activity_table(activities: impl IntoIterator<Item = PrintableActivityItem>) {
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
        col_name.push(activity.display_name().into());
        col_descr.push(description);
        col_wbs.push(wbs);
    }

    print_smart_table! {
        "Name" => col_name,
        "WBS" => col_wbs,
        "Default Description" => col_descr,
    };
}

fn print_collapsed_activity_table(hierarchy: ActivityCategory) {
    let mut leafs: Vec<_> = hierarchy.leafs.into_values().collect();
    let mut branch_names: Vec<_> = hierarchy.branches.into_keys().collect();
    leafs.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    branch_names.sort_unstable();

    let mut col_name: Vec<Rc<str>> = Vec::new();
    let mut col_wbs: Vec<Rc<str>> = Vec::new();
    let mut col_descr: Vec<Rc<str>> = Vec::new();
    let none_value: Rc<str> = NONE_PRINT_VALUE.into();
    for branch in branch_names {
        col_name.push(format!("{}/", branch).into());
        col_wbs.push(none_value.clone());
        col_descr.push(none_value.clone());
    }
    for leaf in leafs {
        let description = match leaf.description() {
            Some(d) => Rc::from(d),
            None => none_value.clone(),
        };
        col_name.push(leaf.name().into());
        col_wbs.push(leaf.wbs().into());
        col_descr.push(description);
    }

    print_smart_table! {
        "Name" => col_name,
        "WBS" => col_wbs,
        "Default Description" => col_descr,
    };
}

fn get_all_trackable_activities() -> Result<Vec<Activity>> {
    let path = files::get_activity_file_path()?;
    if !fs::exists(&path)? {
        return Ok(Vec::new());
    }
    let mut activities = fs::read_to_string(path)?
        .lines()
        .map(Activity::from_str)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    activities.push(Activity::builtin_idle());
    Ok(activities)
}
