use std::str::FromStr;

use color_eyre::{Report, eyre::format_err};
use nom::{IResult, Parser, bytes::complete::take_while1};

#[derive(Debug, Clone)]
pub enum LastValue {
    SingleEntries(usize),
    Hours(usize),
    Days(usize),
    Months(usize),
}

impl FromStr for LastValue {
    type Err = Report;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hour" => return Ok(LastValue::Hours(1)),
            "day" => return Ok(LastValue::Days(1)),
            "month" => return Ok(LastValue::Months(1)),
            _ => (),
        }
        let (postfix, number) = parse_number(s).map_err(|e| format_err!("Invalid number: {e}"))?;
        if number < 1 {
            return Err(format_err!("Must be a positive number"));
        }
        match postfix.to_lowercase().as_str() {
            "" => Ok(LastValue::SingleEntries(number)),
            "h" | "hour" | "hours" => Ok(LastValue::Hours(number)),
            "d" | "day" | "days" => Ok(LastValue::Days(number)),
            "m" | "month" | "months" => Ok(LastValue::Months(number)),
            _ => Err(format_err!("Invalid postfix '{postfix}'")),
        }
    }
}

fn parse_number(input: &str) -> IResult<&str, usize> {
    take_while1(|c: char| c.is_ascii_digit())
        .map_res(|s: &str| s.parse::<usize>())
        .parse(input)
}
