use std::collections::HashMap;

use serde::Deserialize;

use crate::format_string::FormatString;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    employee_name: String,
    employee_number: String,
    cost_center: String,
    performance_type: String,
    accounting_cycle: String,
    default_attendance: String,

    output: OutputConfig,
    attendance_types: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutputConfig {
    upload_destination: String,
    file_name_format: FormatString,
    keys: Vec<String>,
    values: Vec<FormatString>,
    delimeter: String,
}
