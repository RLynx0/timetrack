use std::path::PathBuf;

use color_eyre::eyre::Result;

use crate::environment::Environment;

const DEFAULT_CONFIG_FILENAME: &str = "config.toml";
const ACTIVITY_FILE_NAME: &str = "activities";
const ENTRY_FILE_NAME: &str = "entries";

pub fn get_entry_file_path(env: &Environment) -> Result<PathBuf> {
    Ok(env.data_home()?.join(ENTRY_FILE_NAME))
}

pub fn get_activity_file_path(env: &Environment) -> Result<PathBuf> {
    Ok(env.config_home()?.join(ACTIVITY_FILE_NAME))
}

pub fn get_main_config_path(env: &Environment) -> Result<PathBuf> {
    Ok(env.config_home()?.join(DEFAULT_CONFIG_FILENAME))
}
