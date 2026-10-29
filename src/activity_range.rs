use std::str::FromStr;

use color_eyre::{
    Report,
    eyre::{Result, format_err},
};

use crate::time_window::{TimePoint, TimeWindow};

#[derive(Debug, Clone)]
pub enum ActivityRange {
    Count(usize),
    Window(TimeWindow),
}

impl FromStr for ActivityRange {
    type Err = Report;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if let Some((from, to)) = input.split_once("..") {
            if to.contains("..") {
                return Err(format_err!("A range may contain only one '..'"));
            }
            let from = parse_endpoint(from)?;
            let to = parse_endpoint(to)?;
            if from.as_ref().is_some_and(TimePoint::is_relative)
                && to.as_ref().is_some_and(|point| !point.is_relative())
            {
                return Err(format_err!(
                    "A relative range cannot end at an absolute date; use absolute dates for both endpoints"
                ));
            }
            return Ok(Self::Window(TimeWindow { from, to }));
        }

        if input.bytes().all(|byte| byte.is_ascii_digit()) && !input.is_empty() {
            return Ok(Self::Count(input.parse()?));
        }

        let point: TimePoint = input.parse()?;
        if point.is_relative() {
            Ok(Self::Window(TimeWindow {
                from: Some(point),
                to: None,
            }))
        } else {
            Ok(Self::Window(TimeWindow {
                from: Some(point.clone()),
                to: Some(point),
            }))
        }
    }
}

fn parse_endpoint(input: &str) -> Result<Option<TimePoint>> {
    if input.is_empty() {
        return Ok(None);
    }
    // Four digits are a calendar year here; other bare integers are counts.
    if input.bytes().all(|byte| byte.is_ascii_digit()) && input.len() != 4 {
        return Err(format_err!(
            "Counts are only valid by themselves; use a time unit or date in a range"
        ));
    }
    Ok(Some(input.parse()?))
}

#[derive(Debug, Clone)]
pub enum InLast {
    Hours(i64),
    Days(i64),
    Weeks(i64),
    Months(i64),
}

impl FromStr for InLast {
    type Err = Report;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let split_at = input
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(input.len());
        let (number, unit) = input.split_at(split_at);
        let count = if number.is_empty() {
            0
        } else {
            number.parse::<i64>()?
        };
        match unit.to_ascii_lowercase().as_str() {
            "h" | "hour" | "hours" => Ok(Self::Hours(count)),
            "d" | "day" | "days" => Ok(Self::Days(count)),
            "w" | "week" | "weeks" => Ok(Self::Weeks(count)),
            "m" | "month" | "months" => Ok(Self::Months(count)),
            _ => Err(format_err!(
                "Invalid relative time '{input}'; use a count and unit (e.g. 3days), or a calendar bucket"
            )),
        }
    }
}
