use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::format_string::FormatString;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub employee_name: String,
    pub employee_number: String,
    pub cost_center: String,
    pub performance_type: String,
    pub accounting_cycle: String,
    pub default_attendance: String,

    pub output: OutputConfig,
    pub attendance_types: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub upload_destination: String,
    pub file_name_format: FormatString,
    pub keys: Vec<String>,
    pub values: Vec<FormatString>,
    pub delimeter: String,
}
