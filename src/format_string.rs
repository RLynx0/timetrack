use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
};

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{is_not, tag, take_while1},
    combinator::{opt, recognize},
    multi::{many0, many1},
    sequence::{delimited, pair, preceded},
};
use serde::{Deserialize, de::Visitor};

#[derive(Debug, Clone)]
pub struct FormatString {
    pub(crate) parts: Vec<FormatStringPart>,
}

impl FormatString {
    pub(crate) fn evaluate<'a, S>(
        &'a self,
        variables: &'a HashMap<&str, S>,
    ) -> Result<String, EvalError<'a>>
    where
        S: AsRef<str>,
    {
        let mut buffer = String::new();
        for part in &self.parts {
            match part {
                FormatStringPart::Literal(string) => buffer.push_str(string),
                FormatStringPart::Variable(variable) => match variables.get(variable.as_str()) {
                    Some(value) => buffer.push_str(value.as_ref()),
                    None => {
                        return Err(EvalError::VarNotFound {
                            provided: variables.keys().copied().collect(),
                            requested: variable,
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
                    Ok(("", parsed)) => Ok(parsed),
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
        provided: Vec<&'a str>,
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
    alt((parse_part_variable, parse_part_literal)).parse(input)
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
        take_while1(|c: char| c.is_ascii_alphabetic() || c == '_'),
        opt(take_while1(|c: char| c.is_ascii_alphabetic() || c == '_')),
    ))
    .parse(input)
}

fn parse_part_literal(input: &str) -> IResult<&str, FormatStringPart> {
    many1(alt((is_not("$"), preceded(tag("$"), tag("$")))))
        .map(|s: Vec<&str>| FormatStringPart::Literal(s.join("")))
        .parse(input)
}
