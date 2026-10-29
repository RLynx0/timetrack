use std::collections::HashMap;
use std::fs;

use color_eyre::{
    Section,
    eyre::{Context, Result},
};
use serde::{Deserialize, Serialize};

use crate::format_string::FormatString;
use crate::{environment::Environment, files};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub employee_name: String,
    pub employee_number: String,
    pub cost_center: String,
    pub performance_type: String,
    pub accounting_cycle: String,
    pub default_attendance: String,

    pub generate: GenerateConfig,
    pub attendance_types: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateConfig {
    pub upload_destination: String,
    pub file_name_format: FormatString,
    pub keys: Vec<String>,
    pub values: Vec<FormatString>,
    pub delimiter: String,
    #[serde(default = "default_generate_range")]
    pub range: String,
}

impl Config {
    pub fn load(env: &Environment) -> Result<Self> {
        let config_path = files::get_main_config_path(env)?;
        if fs::exists(&config_path)? {
            let config_str = fs::read_to_string(config_path)?;
            Ok(toml::from_str(&config_str)?)
        } else {
            Err(color_eyre::eyre::format_err!(
                "{config_path:?} does not exist"
            ))
            .wrap_err("Failed to load configuration")
            .with_note(|| "`make-config` can help you create a configuration file")
        }
    }

    pub fn attendance_label(&self, attendance: &str) -> String {
        match self.attendance_types.get(attendance) {
            Some(hint) if !hint.trim().is_empty() => format!("{attendance} ({hint})"),
            _ => attendance.to_string(),
        }
    }
}

fn default_generate_range() -> String {
    "month".into()
}
