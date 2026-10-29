use std::path::PathBuf;

use color_eyre::eyre::Result;

use crate::environment::Environment;

const DEFAULT_CONFIG_FILENAME: &str = "config.toml";
const ACTIVITY_FILE_NAME: &str = "activities";
const ENTRY_FILE_NAME: &str = "entries";

pub fn get_entry_file_path(env: &Environment) -> Result<PathBuf> {
    let mut path = env.data_home()?;
    path.push(ENTRY_FILE_NAME);
    Ok(path)
}

pub fn get_activity_file_path(env: &Environment) -> Result<PathBuf> {
    let mut path = env.config_home()?;
    path.push(ACTIVITY_FILE_NAME);
    Ok(path)
}

pub fn get_main_config_path(env: &Environment) -> Result<PathBuf> {
    let mut path = env.config_home()?;
    path.push(DEFAULT_CONFIG_FILENAME);
    Ok(path)
}
