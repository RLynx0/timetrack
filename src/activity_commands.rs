use std::{fs, io::Write, rc::Rc, str::FromStr};

use color_eyre::{
    Section,
    eyre::{Result, format_err},
};

use crate::{
    NONE_PRINT_VALUE, files, opt, print_smart_table,
    trackable::{Activity, BUILTIN_ACTIVITY_IDLE_NAME},
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
    let path = files::get_activity_dir_path()?;
    let mut col_name: Vec<Rc<str>> = Vec::new();
    let mut col_wbs: Vec<Rc<str>> = Vec::new();
    let mut col_description: Vec<Rc<str>> = Vec::new();
    let none_value: Rc<str> = Rc::from(NONE_PRINT_VALUE);
    for child in fs::read_dir(&path)? {
        let sub_path = child?.path();
        if sub_path.is_file() {
            let act_str = &fs::read_to_string(&sub_path)?;
            let activity = Activity::from_str(act_str)?;
            let description = match activity.description() {
                Some(descr) => Rc::from(descr),
                None => none_value.clone(),
            };
            col_name.push(activity.name().into());
            col_wbs.push(activity.wbs().into());
            col_description.push(description);
        } else {
            let stripped = sub_path.strip_prefix(&path)?;
            let name = stripped
                .to_str()
                .ok_or(format_err!("could not convert {stripped:?} to string"))?;
            col_name.push(format!("{name}/").into());
            col_wbs.push(none_value.clone());
            col_description.push(none_value.clone());
        }
    }

    print_smart_table! {
        "Name" => col_name,
        "WBS" => col_wbs,
        "Description" => col_description,
    };

    Ok(())
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
