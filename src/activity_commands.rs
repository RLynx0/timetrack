use std::{fs, io::Write, path::PathBuf, rc::Rc, str::FromStr};

use color_eyre::{
    Section,
    eyre::{Result, format_err},
};

use crate::{
    NONE_PRINT_VALUE, files, opt, print_smart_table,
    trackable::{Activity, ActivityCategory, ActivityItem, BUILTIN_ACTIVITY_IDLE_NAME},
};

pub fn set_activity(set_opts: &opt::SetActivity) -> Result<()> {
    let name = set_opts.name.trim();
    if name == BUILTIN_ACTIVITY_IDLE_NAME {
        return Err(format_err!(
            "{BUILTIN_ACTIVITY_IDLE_NAME} is a builtin activity and can't be overwritten"
        ));
    }
    let mut path = files::get_activity_dir_path()?;
    path.push(name);
    if path.is_dir() {
        return Err(format_err!("{path:?} is an activity category"));
    }
    if !set_opts.force && fs::exists(&path)? {
        return Err(format_err!("{path:?} already exists")
            .with_note(|| "Use --force to overwrite existing activities"));
    }
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let wbs = &set_opts.wbs;
    let description = set_opts.description.as_deref();
    let activity = Activity::new(name, wbs, description);
    let mut file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    writeln!(file, "{activity}")?;
    println!("Saved trackable '{}'", activity.name());
    Ok(())
}

pub fn list_activities(opts: &opt::ListActivities) -> Result<()> {
    let items = read_activity_hierarchy(opts.name.as_deref(), opts.expand)?
        .into_iter()
        .flat_map(flatten_activity_item)
        .collect::<Vec<_>>();

    if opts.raw {
        for item in items {
            println!("{item}");
        }
    } else {
        print_activity_table(&items);
    }

    Ok(())
}

fn print_activity_table(items: &[ActivityItem]) {
    let mut col_name: Vec<Rc<str>> = Vec::new();
    let mut col_wbs: Vec<Rc<str>> = Vec::new();
    let mut col_description: Vec<Rc<str>> = Vec::new();
    let none_value: Rc<str> = Rc::from(NONE_PRINT_VALUE);
    for item in items {
        match item {
            ActivityItem::Leaf(activity) => {
                let description = match activity.description() {
                    Some(desc) => desc.into(),
                    None => none_value.clone(),
                };
                col_description.push(description);
                col_name.push(activity.name().into());
                col_wbs.push(activity.wbs().into());
            }
            ActivityItem::Category(category) => {
                col_name.push(format!("{}/", category.name).into());
                col_description.push(none_value.clone());
                col_wbs.push(none_value.clone());
            }
        }
    }

    print_smart_table! {
        "Name" => col_name,
        "WBS" => col_wbs,
        "Description" => col_description,
    };
}

fn read_activity_hierarchy(root_name: Option<&str>, recursive: bool) -> Result<Vec<ActivityItem>> {
    let root = root_name.unwrap_or_default();
    let mut path = files::get_activity_dir_path()?;
    path.push(root);
    if !path.exists() {
        return Err(format_err!("{root} does not exist"));
    }
    if !path.is_dir() {
        return Err(format_err!("{path:?} is not an activity category"));
    }

    let mut items = Vec::new();
    for child in fs::read_dir(&path)? {
        let sub_path = child?.path();
        if sub_path.is_file() {
            let act_str = &fs::read_to_string(&sub_path)?;
            let activity = Activity::from_str(act_str)?;
            items.push(ActivityItem::Leaf(activity));
        } else {
            let stripped = sub_path.strip_prefix(&path)?;
            let name: Rc<str> = stripped
                .to_str()
                .ok_or(format_err!("could not convert {stripped:?} to string"))?
                .into();
            let children = recursive
                .then_some(read_activity_hierarchy(Some(&name), recursive)?)
                .unwrap_or_default();
            items.push(ActivityItem::Category(ActivityCategory { name, children }));
        }
    }

    Ok(items)
}
fn flatten_activity_item(item: ActivityItem) -> Vec<ActivityItem> {
    let mut flattened = Vec::new();
    match item {
        ActivityItem::Category(cat) if !cat.children.is_empty() => {
            flattened.extend(cat.children.into_iter().flat_map(flatten_activity_item))
        }
        leaf => flattened.push(leaf),
    }
    flattened
}

pub fn read_activity(name: &str) -> Result<Activity> {
    let mut path = files::get_activity_dir_path()?;
    path.push(name);
    if path.is_dir() {
        return Err(format_err!("{path:?} is an activity category"));
    }
    if !path.exists() {
        return Err(format_err!("{name} does not exist yet"));
    }
    Ok(Activity::from_str(&fs::read_to_string(&path)?)?)
}
