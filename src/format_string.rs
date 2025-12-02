use std::{
    collections::{HashMap, hash_map::Keys},
    error::Error,
    fmt::Display,
};

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::{
        complete::{is_not, tag, take_while_m_n},
        take_while,
    },
    combinator::recognize,
    multi::many0,
    sequence::{delimited, pair, preceded},
};
use serde::{Deserialize, de::Visitor};

#[derive(Debug, Clone)]
pub struct FormatString {
    pub(crate) parts: Vec<FormatStringPart>,
}

impl FormatString {
    pub(crate) fn evaluate<'a>(
        &'a self,
        variables: &'a HashMap<String, String>,
    ) -> Result<String, EvalError<'a>> {
        let mut buffer = String::new();
        for part in &self.parts {
            match part {
                FormatStringPart::Literal(string) => buffer.push_str(string),
                FormatStringPart::Variable(variable) => match variables.get(variable) {
                    Some(value) => buffer.push_str(value),
                    None => {
                        return Err(EvalError::VarNotFound {
                            requested: variable,
                            provided: variables.keys(),
                        });
                    }
                },
            }
        }
        Ok(buffer)
    }
}
impl<'de> Deserialize<'de> for FormatString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FormatStringVisitor;
        impl<'de> Visitor<'de> for FormatStringVisitor {
            type Value = FormatString;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a format string that can include bash-style variables")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match parse_format_string(v) {
                    Ok((rem, parsed)) if rem.is_empty() => Ok(parsed),
                    Ok((rem, _)) => Err(E::custom(format!("failed at '{rem}'"))),
                    Err(e) => Err(E::custom(e)),
                }
            }
        }

        deserializer.deserialize_str(FormatStringVisitor)
    }
}

#[derive(Debug, Clone)]
pub enum FormatStringPart {
    Literal(String),
    Variable(String),
}

#[derive(Debug, Clone)]
pub enum EvalError<'a> {
    VarNotFound {
        requested: &'a str,
        provided: Keys<'a, String, String>,
    },
}
impl<'a> Error for EvalError<'a> {}
impl<'a> Display for EvalError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::VarNotFound {
                requested,
                provided: _,
            } => {
                write!(f, "a variable called '{requested}' was not provided")
            }
        }
    }
}

fn parse_format_string(input: &str) -> IResult<&str, FormatString> {
    let (input, parts) = many0(parse_format_string_part).parse(input)?;
    Ok((input, FormatString { parts }))
}

fn parse_format_string_part(input: &str) -> IResult<&str, FormatStringPart> {
    alt((
        parse_part_variable,
        parse_part_escaped_dollar,
        parse_part_literal,
    ))
    .parse(input)
}

fn parse_part_variable(input: &str) -> IResult<&str, FormatStringPart> {
    alt((
        delimited(tag("${"), parse_varname, tag("}")),
        preceded(tag("$"), parse_varname),
    ))
    .map(|s: &str| FormatStringPart::Variable(s.to_owned()))
    .parse(input)
}

fn parse_varname(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        take_while_m_n(1, 1, |c: char| c.is_ascii_alphabetic() || c == '_'),
        take_while(|c: char| c.is_ascii_alphanumeric() || c == '_'),
    ))
    .parse(input)
}

fn parse_part_escaped_dollar(input: &str) -> IResult<&str, FormatStringPart> {
    tag("$$")
        .map(|_| FormatStringPart::Literal(String::from("$")))
        .parse(input)
}

fn parse_part_literal(input: &str) -> IResult<&str, FormatStringPart> {
    is_not("$")
        .map(|s: &str| FormatStringPart::Literal(s.to_owned()))
        .parse(input)
}
