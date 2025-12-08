use std::str::FromStr;

use color_eyre::{Report, eyre::format_err};
use nom::{IResult, Parser, bytes::complete::take_while1};

#[derive(Debug, Clone)]
pub enum ActivityQuantity {
    SingleEntries(i64),
    Hours(i64),
    Days(i64),
    Weeks(i64),
    Months(i64),
}

impl FromStr for ActivityQuantity {
    type Err = Report;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (postfix, number) = nom::combinator::opt(parse_number)
            .map(|o| o.unwrap_or_default())
            .parse(input)
            .map_err(|e| format_err!("Invalid number: {e}"))?;
        match postfix.to_lowercase().as_str() {
            "" => match number {
                0 => Err(format_err!("Cannot show 0 individual entries")),
                n => Ok(ActivityQuantity::SingleEntries(n)),
            },
            "h" | "hour" | "hours" => Ok(ActivityQuantity::Hours(number)),
            "d" | "day" | "days" => Ok(ActivityQuantity::Days(number)),
            "w" | "week" | "weeks" => Ok(ActivityQuantity::Weeks(number)),
            "m" | "month" | "months" => Ok(ActivityQuantity::Months(number)),
            _ => Err(format_err!("Invalid postfix '{postfix}'")),
        }
    }
}

fn parse_number(input: &str) -> IResult<&str, i64> {
    take_while1(|c: char| c.is_ascii_digit())
        .map_res(|s: &str| s.parse::<i64>())
        .parse(input)
}
