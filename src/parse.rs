use crate::types::{Jqesque, JqesqueError, Operation, PathToken, Separator};
use nom::{
    branch::alt,
    bytes::complete::{escaped_transform, is_not, take_while1},
    character::complete::{char, digit1, none_of, one_of},
    combinator::{all_consuming, map, map_res, opt},
    error::VerboseError,
    multi::{many1, separated_list1},
    sequence::delimited,
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
/// Returns a `Jqesque` structure if successful, or a `JqesqueError` if parsing fails.
pub fn parse_input(input: &str, separator: Separator) -> Result<Jqesque, JqesqueError> {
    let sep_char = separator.as_char();
    let res = all_consuming(|i| jqesque(i, sep_char))(input);
    match res {
        Ok((_, jqesque)) => Ok(jqesque),
        Err(err) => Err(JqesqueError::NomError(format!("{}", err))),
    }
}

fn jqesque(input: &str, separator: char) -> Res<&str, Jqesque> {
    let (input, operation) = opt(operation_prefix)(input)?;
    let operation = operation.unwrap_or(Operation::Auto);

    let (input, (tokens, value)) = assignment(input, separator, &operation)?;

    Ok((
        input,
        Jqesque {
            operation,
            tokens,
            value,
        },
    ))
}

fn operation_prefix(input: &str) -> Res<&str, Operation> {
    let (input, op_char) = one_of(Operation::operators())(input)?;
    let operation =
        Operation::from_operator(op_char).expect("operator should be valid since we used one_of");
    Ok((input, operation))
}

fn assignment<'a>(
    input: &'a str,
    separator: char,
    operation: &Operation,
) -> Res<&'a str, (Vec<PathToken>, Option<Value>)> {
    let (input, tokens) = path(input, separator)?;

    let (input, value_opt) = match operation {
        Operation::Remove => (input, None),
        _ => {
            let (input, _) = char('=')(input)?;
            let (input, _) = opt(char(' '))(input)?;
            let (input, value) = json_value(input)?;
            (input, Some(value))
        }
    };

    Ok((input, (tokens, value_opt)))
}

fn path(input: &str, separator: char) -> Res<&str, Vec<PathToken>> {
    let (input, token_vecs) =
        separated_list1(char(separator), alt((array_access, key_segment)))(input)?;

    let tokens = token_vecs.into_iter().flatten().collect();

    Ok((input, tokens))
}

fn key_segment(input: &str) -> Res<&str, Vec<PathToken>> {
    map(alt((quoted_string, valid_identifier)), |s: String| {
        vec![PathToken::Key(s)]
    })(input)
}

fn array_access(input: &str) -> Res<&str, Vec<PathToken>> {
    let (input, key_opt) = opt(alt((quoted_string, valid_identifier)))(input)?;

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
