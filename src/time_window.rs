use std::str::FromStr;

use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike};
use color_eyre::eyre::{Result, format_err};

#[derive(Debug, Clone)]
pub enum TimePoint {
    Relative(Relative),
    Year(i32),
    Month(i32, u32),
    Day(NaiveDate),
}

#[derive(Clone, Copy)]
pub(crate) enum Direction {
    Backward,
    Forward,
}

#[derive(Debug, Clone, Copy)]
pub enum Unit {
    Hour,
    Day,
    Week,
    Month,
    Year,
}

impl Unit {
    fn floor(self, reference: NaiveDateTime) -> Result<NaiveDateTime> {
        let date = reference.date();
        let midnight = date.and_time(NaiveTime::MIN);
        match self {
            Self::Hour => Ok(date.and_hms_opt(reference.hour(), 0, 0).unwrap()),
            Self::Day => Ok(midnight),
            Self::Week => midnight
                .checked_sub_signed(TimeDelta::days(i64::from(
                    date.weekday().num_days_from_monday(),
                )))
                .ok_or_else(|| format_err!("Time point out of range")),
            Self::Month => Ok(month_start(date.year(), date.month(), 0)?.and_time(NaiveTime::MIN)),
            Self::Year => Ok(month_start(date.year(), 1, 0)?.and_time(NaiveTime::MIN)),
        }
    }

    fn step(self, start: NaiveDateTime, count: i64) -> Result<NaiveDateTime> {
        let delta = match self {
            Self::Hour => TimeDelta::try_hours(count),
            Self::Day => TimeDelta::try_days(count),
            Self::Week => TimeDelta::try_weeks(count),
            Self::Month | Self::Year => {
                let months = count
                    .checked_mul(if matches!(self, Self::Year) { 12 } else { 1 })
                    .ok_or_else(|| format_err!("Month out of range"))?;
                return Ok(
                    month_start(start.year(), start.month(), months)?.and_time(NaiveTime::MIN)
                );
            }
        };
        delta
            .and_then(|delta| start.checked_add_signed(delta))
            .ok_or_else(|| format_err!("Time point out of range"))
    }
}

#[derive(Debug, Clone)]
pub struct Relative {
    pub count: i64,
    pub unit: Unit,
}

impl FromStr for Relative {
    type Err = color_eyre::Report;

    fn from_str(input: &str) -> Result<Self> {
        let split_at = input
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(input.len());
        let (number, unit) = input.split_at(split_at);
        let count = if number.is_empty() {
            0
        } else {
            number.parse()?
        };
        let unit = match unit.to_ascii_lowercase().as_str() {
            "h" | "hour" | "hours" => Unit::Hour,
            "d" | "day" | "days" => Unit::Day,
            "w" | "week" | "weeks" => Unit::Week,
            "m" | "month" | "months" => Unit::Month,
            _ => {
                return Err(format_err!(
                    "Invalid relative time '{input}'; use a count and unit (e.g. 3days)"
                ));
            }
        };
        Ok(Self { count, unit })
    }
}

impl FromStr for TimePoint {
    type Err = color_eyre::Report;

    fn from_str(input: &str) -> Result<Self> {
        let parts: Vec<_> = input.split('-').collect();
        if parts
            .first()
            .is_some_and(|s| s.len() == 4 && s.bytes().all(|c| c.is_ascii_digit()))
            && parts.len() <= 3
            && parts
                .iter()
                .skip(1)
                .all(|s| s.len() == 2 && s.bytes().all(|c| c.is_ascii_digit()))
        {
            let year = parts[0].parse()?;
            let month = parts.get(1).map(|s| s.parse()).transpose()?.unwrap_or(1);
            let day = parts.get(2).map(|s| s.parse()).transpose()?.unwrap_or(1);
            let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
                format_err!("Invalid date '{input}'; use YYYY, YYYY-MM or YYYY-MM-DD")
            })?;
            return Ok(match parts.len() {
                1 => Self::Year(year),
                2 => Self::Month(year, month),
                _ => Self::Day(date),
            });
        }
        if let Ok(relative) = input.parse() {
            return Ok(Self::Relative(relative));
        }
        Err(format_err!(
            "Invalid time point '{input}'; use a relative unit (3days, week) or YYYY, YYYY-MM, YYYY-MM-DD. Counts are only valid by themselves"
        ))
    }
}

fn local_time(time: NaiveDateTime) -> Result<DateTime<Local>> {
    time.and_local_timezone(Local)
        .earliest()
        .ok_or_else(|| format_err!("Local time '{time}' does not exist"))
}

fn month_start(year: i32, month: u32, offset: i64) -> Result<NaiveDate> {
    let index = i64::from(year) * 12 + i64::from(month) - 1;
    let index = index
        .checked_add(offset)
        .ok_or_else(|| format_err!("Month out of range"))?;
    let year = i32::try_from(index.div_euclid(12))?;
    NaiveDate::from_ymd_opt(year, index.rem_euclid(12) as u32 + 1, 1)
        .ok_or_else(|| format_err!("Month out of range"))
}

impl TimePoint {
    pub(crate) fn is_relative(&self) -> bool {
        matches!(self, Self::Relative(_))
    }

    pub(crate) fn bounds(
        &self,
        reference: DateTime<Local>,
        direction: Direction,
    ) -> Result<(DateTime<Local>, DateTime<Local>)> {
        let midnight = |date: NaiveDate| date.and_time(NaiveTime::MIN);
        let (unit, start) = match self {
            Self::Relative(relative) => {
                let sign = match direction {
                    Direction::Backward => -1,
                    Direction::Forward => 1,
                };
                let count = relative
                    .count
                    .checked_mul(sign)
                    .ok_or_else(|| format_err!("Time point out of range"))?;
                (
                    relative.unit,
                    relative
                        .unit
                        .step(relative.unit.floor(reference.naive_local())?, count)?,
                )
            }
            Self::Year(year) => (Unit::Year, midnight(month_start(*year, 1, 0)?)),
            Self::Month(year, month) => (Unit::Month, midnight(month_start(*year, *month, 0)?)),
            Self::Day(date) => (Unit::Day, midnight(*date)),
        };
        let end = unit.step(start, 1)?;
        Ok((local_time(start)?, local_time(end)?))
    }
}

#[derive(Debug, Clone)]
pub struct TimeWindow {
    from: Option<TimePoint>,
    to: Option<TimePoint>,
}

impl FromStr for TimeWindow {
    type Err = color_eyre::Report;

    fn from_str(input: &str) -> Result<Self> {
        let (from, to) = if let Some((from, to)) = input.split_once("..") {
            if to.contains("..") {
                return Err(format_err!("A range may contain only one '..'"));
            }
            let endpoint = |value: &str| {
                if value.is_empty() {
                    Ok(None)
                } else {
                    value.parse::<TimePoint>().map(Some)
                }
            };
            (endpoint(from)?, endpoint(to)?)
        } else {
            let point: TimePoint = input.parse()?;
            let to = if point.is_relative() {
                None
            } else {
                Some(point.clone())
            };
            (Some(point), to)
        };
        if matches!((&from, &to), (Some(from), Some(to)) if from.is_relative() && !to.is_relative())
        {
            return Err(format_err!(
                "A relative range cannot end at an absolute date; use absolute dates for both endpoints"
            ));
        }
        Ok(Self { from, to })
    }
}

impl TimeWindow {
    pub fn resolve(
        &self,
        now: DateTime<Local>,
    ) -> Result<(Option<DateTime<Local>>, DateTime<Local>)> {
        let start = self
            .from
            .as_ref()
            .map(|point| {
                point
                    .bounds(now, Direction::Backward)
                    .map(|bounds| bounds.0)
            })
            .transpose()?;
        let end = match (&self.from, &self.to, start) {
            (_, None, _) => now,
            (Some(from), Some(to), Some(start)) if !from.is_relative() && to.is_relative() => {
                to.bounds(start, Direction::Forward)?.1
            }
            (_, Some(to), _) => to.bounds(now, Direction::Backward)?.1,
        }
        .min(now);
        if start.is_some_and(|start| start >= end) {
            return Err(format_err!(
                "Empty or inverted range; the start must precede the end (and now)"
            ));
        }
        Ok((start, end))
    }
}

#[cfg(test)]
mod tests;
