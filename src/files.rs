use std::{env, path::PathBuf};

use color_eyre::eyre::Result;

const FS_SCOPE_NAME: &str = "timetrack";
const DEFAULT_CONFIG_FILENAME: &str = "config.toml";
const ACTIVITY_FILE_NAME: &str = "activities";
const ENTRY_FILE_NAME: &str = "entries";
const CONFIG_HOME_VAR: &str = "TIMETRACK_HOME";
const DATA_HOME_VAR: &str = "TIMETRACK_DATA_HOME";

pub fn get_entry_file_path() -> Result<PathBuf> {
    let mut path = get_data_home()?;
    path.push(ENTRY_FILE_NAME);
    Ok(path)
}

pub fn get_activity_file_path() -> Result<PathBuf> {
    let mut path = get_config_home()?;
    path.push(ACTIVITY_FILE_NAME);
    Ok(path)
}

pub fn get_main_config_path() -> Result<PathBuf> {
    let mut path = get_config_home()?;
    path.push(DEFAULT_CONFIG_FILENAME);
    Ok(path)
}

fn get_config_home() -> Result<PathBuf> {
    env::var(CONFIG_HOME_VAR).map(PathBuf::from).or_else(|_| {
        let mut path = get_xdg_config_home()?;
        path.push(FS_SCOPE_NAME);
        Ok(path)
    })
}
fn get_xdg_config_home() -> Result<PathBuf> {
    env::var("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|_| {
        Ok(PathBuf::from_iter([
            env::var("HOME")?,
            String::from(".config"),
        ]))
    })
}

fn get_data_home() -> Result<PathBuf> {
    env::var(DATA_HOME_VAR).map(PathBuf::from).or_else(|_| {
        let mut path = get_xdg_data_home()?;
        path.push(FS_SCOPE_NAME);
        Ok(path)
    })
}
fn get_xdg_data_home() -> Result<PathBuf> {
    env::var("XDG_DATA_HOME").map(PathBuf::from).or_else(|_| {
        Ok(PathBuf::from_iter([
            env::var("HOME")?,
            String::from(".local"),
            String::from("share"),
        ]))
    })
}
