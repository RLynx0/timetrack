use std::str::FromStr;

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeDelta, TimeZone, Timelike};
use color_eyre::{
    Report,
    eyre::{Result, format_err},
};
use nom::{IResult, Parser, bytes::complete::take_while1};

#[derive(Debug, Clone)]
pub enum ActivityRange {
    Count(i64),
    Timeframe(InLast),
}
impl FromStr for ActivityRange {
    type Err = Report;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (postfix, number) = nom::combinator::opt(parse_number)
            .map(|o| o.unwrap_or_default())
            .parse(input)
            .map_err(|e| format_err!("Invalid number: {e}"))?;
        match postfix.to_lowercase().as_str() {
            "" => Ok(ActivityRange::Count(number)),
            "h" | "hour" | "hours" => Ok(ActivityRange::Timeframe(InLast::Hours(number))),
            "d" | "day" | "days" => Ok(ActivityRange::Timeframe(InLast::Days(number))),
            "w" | "week" | "weeks" => Ok(ActivityRange::Timeframe(InLast::Weeks(number))),
            "m" | "month" | "months" => Ok(ActivityRange::Timeframe(InLast::Months(number))),
            _ => Err(format_err!("Invalid postfix '{postfix}'")),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InLast {
    Hours(i64),
    Days(i64),
    Weeks(i64),
    Months(i64),
}
impl InLast {
    pub fn back_from(&self, now: &DateTime<Local>) -> DateTime<Local> {
        match self {
            InLast::Hours(h) => {
                now.with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap()
                    - TimeDelta::hours(*h)
            }
            InLast::Days(d) => {
                now.with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap()
                    - TimeDelta::days(*d)
            }
            InLast::Weeks(w) => {
                let days_since_monday = now.weekday().num_days_from_monday();
                now.with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap()
                    - TimeDelta::days(*w * 7 + days_since_monday as i64)
            }
            InLast::Months(m) => {
                let month = now.month0() as i64;
                let month_delta = month - m;
                let year = now.year();
                let new_year = year + month_delta.div_euclid(12) as i32;
                let new_month = month_delta.rem_euclid(12) as u32 + 1;
                let naive = NaiveDate::from_ymd_opt(new_year, new_month, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap();

                match Local.from_local_datetime(&naive) {
                    chrono::offset::LocalResult::Single(s) => s,
                    chrono::offset::LocalResult::Ambiguous(a, _) => a,
                    chrono::offset::LocalResult::None => {
                        let fallback = naive + TimeDelta::hours(1);
                        Local.from_local_datetime(&fallback).unwrap()
                    }
                }
            }
        }
    }
}

fn parse_number(input: &str) -> IResult<&str, i64> {
    take_while1(|c: char| c.is_ascii_digit())
        .map_res(|s: &str| s.parse::<i64>())
        .parse(input)
}
