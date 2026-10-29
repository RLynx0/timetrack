use std::str::FromStr;

use color_eyre::{Report, eyre::Result};

use crate::time_window::TimeWindow;

#[derive(Debug, Clone)]
pub enum ActivityRange {
    Count(usize),
    Window(TimeWindow),
}

impl FromStr for ActivityRange {
    type Err = Report;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if input.bytes().all(|byte| byte.is_ascii_digit()) && !input.is_empty() {
            return Ok(Self::Count(input.parse()?));
        }
        Ok(Self::Window(input.parse()?))
    }
}
