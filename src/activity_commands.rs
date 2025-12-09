use std::{fs, io::Write, str::FromStr};

use color_eyre::{
    Section,
    eyre::{Result, format_err},
};

use crate::{files, opt, trackable::Activity};

pub fn set_activity(set_opts: &opt::SetActivity) -> Result<()> {
    let mut path = files::get_activity_dir_path()?;
    path.push(&set_opts.name);
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
    let activity = Activity::new(
        &set_opts.name,
        &set_opts.wbs,
        set_opts.description.as_deref(),
    );
    let mut file = fs::OpenOptions::new()
        .truncate(true)
        .write(true)
        .open(path)?;
    writeln!(file, "{activity}")?;
    println!("Written {}", activity.name());
    Ok(())
}

pub fn list_activities(opts: &opt::ListActivities) -> Result<()> {
    todo!("read and list activities")
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
