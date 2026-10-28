use std::str::FromStr;

use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike};
use color_eyre::eyre::{Result, format_err};

use crate::activity_range::{ActivityRange, InLast};

#[derive(Debug, Clone)]
pub enum TimePoint {
    Relative(InLast),
    Year(i32),
    Month(i32, u32),
    Day(NaiveDate),
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
        if let Ok(ActivityRange::Timeframe(relative)) = input.parse() {
            return Ok(Self::Relative(relative));
        }
        Err(format_err!(
            "Invalid time point '{input}'; use a relative unit (1m, week) or YYYY, YYYY-MM, YYYY-MM-DD"
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
    pub fn bounds(&self, now: DateTime<Local>) -> Result<(DateTime<Local>, DateTime<Local>)> {
        let date = now.date_naive();
        let midnight = |date: NaiveDate| date.and_time(NaiveTime::MIN);
        let shift = |time: NaiveDateTime, delta: TimeDelta| {
            time.checked_add_signed(delta)
                .ok_or_else(|| format_err!("Time point out of range"))
        };
        let (start, end) = match self {
            Self::Relative(InLast::Months(n)) => {
                let offset = n
                    .checked_neg()
                    .ok_or_else(|| format_err!("Month out of range"))?;
                let start = month_start(now.year(), now.month(), offset)?;
                (
                    midnight(start),
                    midnight(month_start(start.year(), start.month(), 1)?),
                )
            }
            Self::Relative(relative) => {
                let (base, count, unit) = match relative {
                    InLast::Hours(n) => (
                        date.and_hms_opt(now.hour(), 0, 0).unwrap(),
                        *n,
                        TimeDelta::hours(1),
                    ),
                    InLast::Days(n) => (midnight(date), *n, TimeDelta::days(1)),
                    InLast::Weeks(n) => (
                        shift(
                            midnight(date),
                            -TimeDelta::days(i64::from(date.weekday().num_days_from_monday())),
                        )?,
                        *n,
                        TimeDelta::weeks(1),
                    ),
                    InLast::Months(_) => unreachable!(),
                };
                let count = i32::try_from(count)?;
                let delta = unit
                    .checked_mul(count)
                    .ok_or_else(|| format_err!("Time point out of range"))?;
                let start = shift(base, -delta)?;
                (start, shift(start, unit)?)
            }
            Self::Year(year) => {
                let next = year
                    .checked_add(1)
                    .ok_or_else(|| format_err!("Year out of range"))?;
                (
                    midnight(month_start(*year, 1, 0)?),
                    midnight(month_start(next, 1, 0)?),
                )
            }
            Self::Month(year, month) => (
                midnight(month_start(*year, *month, 0)?),
                midnight(month_start(*year, *month, 1)?),
            ),
            Self::Day(date) => (
                midnight(*date),
                midnight(
                    date.succ_opt()
                        .ok_or_else(|| format_err!("Date out of range"))?,
                ),
            ),
        };
        Ok((local_time(start)?, local_time(end)?))
    }
}

pub struct TimeWindow {
    pub from: Option<TimePoint>,
    pub to: Option<TimePoint>,
}

impl TimeWindow {
    pub fn resolve(
        &self,
        now: DateTime<Local>,
    ) -> Result<(Option<DateTime<Local>>, DateTime<Local>)> {
        let start = self
            .from
            .as_ref()
            .map(|p| p.bounds(now).map(|b| b.0))
            .transpose()?;
        // Bucket ends are represented by the exclusive start of the next bucket.
        let end = self
            .to
            .as_ref()
            .map(|p| p.bounds(now).map(|b| b.1))
            .transpose()?
            .unwrap_or(now)
            .min(now);
        if start.is_some_and(|start| start >= end) {
            return Err(format_err!(
                "Empty or inverted time window; --from must precede the end of --to (and now)"
            ));
        }
        Ok((start, end))
    }
}

#[cfg(test)]
mod tests;
