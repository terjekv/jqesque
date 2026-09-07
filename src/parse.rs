use crate::types::{Jqesque, JqesqueError, Operation, ParseOptions, PathToken, Separator};
use nom::{
    branch::alt,
    bytes::complete::{escaped_transform, is_not, take_while1},
    character::complete::{char, digit1, none_of, one_of},
    combinator::{all_consuming, map, map_res, opt},
    multi::{many1, separated_list1},
    sequence::delimited,
    IResult, Parser,
};
use nom_language::error::VerboseError;
use serde_json::Value;

type Res<T, U> = IResult<T, U, VerboseError<T>>;

struct Assignment<'a> {
    tokens: Vec<PathToken>,
    value: Option<&'a str>,
    operation: Operation,
}

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
    parse_input_with_options(input, ParseOptions::new(separator))
}

pub fn parse_input_with_options(
    input: &str,
    options: ParseOptions,
) -> Result<Jqesque, JqesqueError> {
    let sep_char = options.separator().as_char();
    let (_, assignment) = all_consuming(|i| jqesque(i, sep_char))
        .parse(input)
        .map_err(|err| JqesqueError::NomError(err.to_string()))?;
    let value = assignment
        .value
        .map(|value| json_value(value, options.strict_json_values_enabled()))
        .transpose()?;
    let jqesque = Jqesque::new(assignment.tokens, value, assignment.operation)?;
    jqesque.validate_limits(
        options.max_path_depth_limit(),
        options.max_array_index_limit(),
    )?;
    Ok(jqesque)
}

fn jqesque(input: &str, separator: char) -> Res<&str, Assignment<'_>> {
    let (input, operation) = opt(operation_prefix).parse(input)?;
    let operation = operation.unwrap_or(Operation::Auto);

    let (input, (tokens, value)) = assignment(input, separator, &operation)?;

    Ok((
        input,
        Assignment {
            tokens,
            value,
            operation,
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
) -> Res<&'a str, (Vec<PathToken>, Option<&'a str>)> {
    let (input, tokens) = path(input, separator)?;

    let (input, value_opt) = match operation {
        Operation::Remove => (input, None),
        _ => {
            let (input, _) = char('=')(input)?;
            let (input, _) = opt(char(' ')).parse(input)?;
            let (input, value) = is_not("")(input)?;
            (input, Some(value))
        }
    };

    Ok((input, (tokens, value_opt)))
}

fn path(input: &str, separator: char) -> Res<&str, Vec<PathToken>> {
    let (input, token_vecs) =
        separated_list1(char(separator), alt((array_access, key_segment))).parse(input)?;

    let tokens = token_vecs.into_iter().flatten().collect();

    Ok((input, tokens))
}

fn key_segment(input: &str) -> Res<&str, Vec<PathToken>> {
    map(alt((quoted_string, valid_identifier)), |s: String| {
        vec![PathToken::Key(s)]
    })
    .parse(input)
}

fn array_access(input: &str) -> Res<&str, Vec<PathToken>> {
    let (input, key_opt) = opt(alt((quoted_string, valid_identifier))).parse(input)?;

    let (input, indices) = many1(delimited(
        char('['),
        map_res(digit1, |s: &str| s.parse::<usize>()),
        char(']'),
    ))
    .parse(input)?;

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
    )
    .parse(input)
}

fn quoted_string(input: &str) -> Res<&str, String> {
    delimited(
        char('"'),
        escaped_transform(none_of("\\\""), '\\', one_of("\\\"nrt")),
        char('"'),
    )
    .parse(input)
}

fn json_value(input: &str, strict_json_values: bool) -> Result<Value, JqesqueError> {
    match serde_json::from_str(input) {
        Ok(value) => Ok(value),
        Err(err) if strict_json_values => Err(JqesqueError::InvalidJsonValueError(err.to_string())),
        Err(_) => Ok(Value::String(input.to_string())),
    }
}
