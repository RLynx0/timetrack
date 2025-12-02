use std::{env, path::PathBuf};

const FS_SCOPE_NAME: &'static str = "timetracker";
const DEFAULT_CONFIG_FILENAME: &'static str = "config.toml";
const ACTIVITY_FILE_NAME: &'static str = "activities";
const ENTRY_FILE_NAME: &'static str = "entries";

pub fn get_data_dir_path() -> anyhow::Result<PathBuf> {
    let mut path = get_xdg_data_home()?;
    path.push(FS_SCOPE_NAME);
    Ok(path)
}

pub fn get_entry_file_path() -> anyhow::Result<PathBuf> {
    let mut path = get_data_dir_path()?;
    path.push(ENTRY_FILE_NAME);
    Ok(path)
}

pub fn get_activity_file_path() -> anyhow::Result<PathBuf> {
    let mut path = get_data_dir_path()?;
    path.push(ACTIVITY_FILE_NAME);
    Ok(path)
}

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    let mut path = get_xdg_config_home()?;
    path.push(FS_SCOPE_NAME);
    path.push(DEFAULT_CONFIG_FILENAME);
    Ok(path)
}

fn get_xdg_config_home() -> anyhow::Result<PathBuf> {
    match env::var("XDG_CONFIG_HOME") {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(_) => Ok(PathBuf::from_iter([
            env::var("HOME")?,
            String::from(".config"),
        ])),
    }
}

fn get_xdg_data_home() -> anyhow::Result<PathBuf> {
    match env::var("XDG_DATA_HOME") {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(_) => Ok(PathBuf::from_iter([
            env::var("HOME")?,
            String::from(".local"),
            String::from("share"),
        ])),
    }
}
