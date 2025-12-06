use std::str::FromStr;

use color_eyre::{Report, eyre::format_err};
use nom::{IResult, Parser, bytes::complete::take_while1};

#[derive(Debug, Clone)]
pub enum LastValue {
    SingleEntries(i64),
    Hours(i64),
    Days(i64),
    Months(i64),
}

impl FromStr for LastValue {
    type Err = Report;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (postfix, number) = nom::combinator::opt(parse_number)
            .map(|o| o.unwrap_or_default())
            .parse(input)
            .map_err(|e| format_err!("Invalid number: {e}"))?;
        match postfix.to_lowercase().as_str() {
            "" => match number {
                0 => Err(format_err!("Cannot show 0 individual entries")),
                n => Ok(LastValue::SingleEntries(n)),
            },
            "h" | "hour" | "hours" => Ok(LastValue::Hours(number)),
            "d" | "day" | "days" => Ok(LastValue::Days(number)),
            "m" | "month" | "months" => Ok(LastValue::Months(number)),
            _ => Err(format_err!("Invalid postfix '{postfix}'")),
        }
    }
}

fn parse_number(input: &str) -> IResult<&str, i64> {
    take_while1(|c: char| c.is_ascii_digit())
        .map_res(|s: &str| s.parse::<i64>())
        .parse(input)
}
