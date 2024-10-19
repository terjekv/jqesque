use crate::types::{Jqesque, ParseError, PathToken, Separator};
use nom::{
    branch::alt,
    bytes::complete::{escaped_transform, is_not, take_while1},
    character::complete::{char, digit1, none_of, one_of},
    combinator::{all_consuming, map, map_res, opt},
    error::VerboseError,
    multi::{many1, separated_list1},
    sequence::{delimited, preceded, separated_pair},
    IResult,
};
use serde_json::Value;

type Res<T, U> = IResult<T, U, VerboseError<T>>;

/// Parses the input string into path tokens and a serde_json::Value.
///
/// ## Arguments
///
/// * `input` - The input string, e.g., "foo.bar[0].baz=true"
/// * `separator` - The separator to use between keys, a Separator enum variant.
///
/// ## Returns
///
/// Returns a `Jqesque` structure if successful, or a `ParseError` if parsing fails.
pub fn parse_input(input: &str, separator: Separator) -> Result<Jqesque, ParseError> {
    let sep_char = separator.as_char();
    let res = all_consuming(|i| assignment(i, sep_char))(input);
    match res {
        Ok((_, (tokens, value))) => Ok(Jqesque { tokens, value }),
        Err(err) => Err(ParseError::NomError(format!("{}", err))),
    }
}

fn assignment(input: &str, separator: char) -> Res<&str, (Vec<PathToken>, Value)> {
    separated_pair(
        |i| path(i, separator),
        char('='),
        preceded(opt(char(' ')), json_value),
    )(input)
}

fn path(input: &str, separator: char) -> Res<&str, Vec<PathToken>> {
    let (input, token_vecs) =
        separated_list1(char(separator), alt((array_access, key_segment)))(input)?;

    // Flatten the vectors into a single Vec<PathToken>
    let tokens = token_vecs.into_iter().flatten().collect();

    Ok((input, tokens))
}

fn key_segment(input: &str) -> Res<&str, Vec<PathToken>> {
    map(alt((quoted_string, valid_identifier)), |s: String| {
        vec![PathToken::Key(s)]
    })(input)
}

fn array_access(input: &str) -> Res<&str, Vec<PathToken>> {
    // Try to parse an optional key
    let (input, key_opt) = opt(alt((quoted_string, valid_identifier)))(input)?;

    // Require at least one index
    let (input, indices) = many1(delimited(
        char('['),
        map_res(digit1, |s: &str| s.parse::<usize>()),
        char(']'),
    ))(input)?;

    let mut tokens = Vec::new();

    if let Some(key) = key_opt {
        tokens.push(PathToken::Key(key));
    }

    for index in indices {
        tokens.push(PathToken::Index(index));
    }

    Ok((input, tokens))
}

fn valid_identifier(input: &str) -> Res<&str, String> {
    map(
        take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '-'),
        |s: &str| s.to_string(),
    )(input)
}

fn quoted_string(input: &str) -> Res<&str, String> {
    delimited(
        char('"'),
        escaped_transform(none_of("\\\""), '\\', one_of("\\\"nrt")),
        char('"'),
    )(input)
}

fn json_value(input: &str) -> Res<&str, Value> {
    map(is_not(""), |s: &str| {
        serde_json::from_str(s).unwrap_or(Value::String(s.to_string()))
    })(input)
}
