use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
    sync::Arc,
};

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{is_not, tag, take_while1},
    combinator::{opt, recognize},
    multi::{many0, many1},
    sequence::{delimited, pair, preceded},
};
use serde::{Deserialize, Serialize, de::Visitor};

#[derive(Debug, Clone)]
pub struct FormatString {
    pub(crate) parts: Vec<FormatStringPart>,
}

impl FormatString {
    pub(crate) fn evaluate<'a, S>(
        &'a self,
        variables: &'a HashMap<&str, S>,
    ) -> Result<String, EvalError>
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
                            provided: variables.keys().copied().map(<Arc<str>>::from).collect(),
                            requested: Arc::from(variable.as_str()),
                        });
                    }
                },
            }
        }
        Ok(buffer)
    }
}
impl Display for FormatString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for part in &self.parts {
            match part {
                FormatStringPart::Variable(v) => write!(f, "${{{}}}", v)?,
                FormatStringPart::Literal(s) => write!(f, "{}", s.replace("$", "$$"))?,
            }
        }
        Ok(())
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
impl Serialize for FormatString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum FormatStringPart {
    Literal(String),
    Variable(String),
}

#[derive(Debug, Clone)]
pub enum EvalError {
    VarNotFound {
        requested: Arc<str>,
        provided: Vec<Arc<str>>,
    },
}
impl Error for EvalError {}
impl Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::VarNotFound {
                requested,
                provided,
            } => {
                write!(f, "a variable called '{requested}' was not provided")?;
                if let Some(similar) = similar_variable(requested, provided) {
                    write!(f, "; a similar variable exists: '{similar}'")?;
                }
                Ok(())
            }
        }
    }
}

fn similar_variable<'a>(requested: &str, provided: &'a [Arc<str>]) -> Option<&'a str> {
    let candidates = || provided.iter().map(AsRef::as_ref);
    if let Some(candidate) = candidates()
        .filter(|candidate: &&str| candidate.eq_ignore_ascii_case(requested))
        .min()
    {
        return Some(candidate);
    }

    // Match rustc's default typo cutoff and restricted Damerau-Levenshtein metric.
    let limit = requested.chars().count().max(3) / 3;
    if let Some((_, candidate)) = candidates()
        .map(|candidate| (strsim::osa_distance(requested, candidate), candidate))
        .filter(|(distance, _)| *distance <= limit)
        .min()
    {
        return Some(candidate);
    }

    fn sorted_words(name: &str) -> Vec<&str> {
        let mut words: Vec<_> = name.split('_').collect();
        words.sort_unstable();
        words
    }
    let requested_words = sorted_words(requested);
    candidates()
        .filter(|candidate| sorted_words(candidate) == requested_words)
        .min()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn missing_variable_message(requested: &str, provided: &[&str]) -> String {
        let format = FormatString {
            parts: vec![FormatStringPart::Variable(requested.to_owned())],
        };
        let variables: HashMap<_, _> = provided.iter().map(|name| (*name, "")).collect();
        format.evaluate(&variables).unwrap_err().to_string()
    }

    #[test]
    fn missing_variables_suggest_nearby_names() {
        for requested in ["mont", "monnth", "monxh", "motnh", "MONTH"] {
            assert_eq!(
                missing_variable_message(requested, &["day", "month", "year"]),
                format!(
                    "a variable called '{requested}' was not provided; a similar variable exists: 'month'"
                )
            );
        }
        assert_eq!(
            missing_variable_message("name_employee", &["employee_name"]),
            "a variable called 'name_employee' was not provided; a similar variable exists: 'employee_name'"
        );
    }

    #[test]
    fn missing_variables_prefer_case_matches_then_nearest_names() {
        assert!(missing_variable_message("MONTH", &["MONT", "month"]).ends_with("'month'"));
        assert!(missing_variable_message("montth", &["months", "month"]).ends_with("'month'"));
        for provided in [&["monty", "month"][..], &["month", "monty"][..]] {
            assert!(missing_variable_message("montx", provided).ends_with("'month'"));
        }
    }

    #[test]
    fn missing_variables_do_not_suggest_unrelated_names() {
        for provided in [&["year", "day"][..], &[][..]] {
            assert_eq!(
                missing_variable_message("description", provided),
                "a variable called 'description' was not provided"
            );
        }
    }
}
