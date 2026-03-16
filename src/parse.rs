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
    let strict_json_values = options.strict_json_values_enabled();
    let res = all_consuming(|i| jqesque(i, sep_char, strict_json_values)).parse(input);
    match res {
        Ok((_, jqesque)) => {
            jqesque.validate_limits(
                options.max_path_depth_limit(),
                options.max_array_index_limit(),
            )?;
            Ok(jqesque)
        }
        Err(err) => {
            let err_msg = format!("{}", err);
            if strict_json_values && err_msg.contains("Invalid JSON value in strict mode:") {
                return Err(JqesqueError::InvalidJsonValueError(err_msg));
            }
            Err(JqesqueError::NomError(err_msg))
        }
    }
}

fn jqesque(input: &str, separator: char, strict_json_values: bool) -> Res<&str, Jqesque> {
    let (input, operation) = opt(operation_prefix).parse(input)?;
    let operation = operation.unwrap_or(Operation::Auto);

    let (input, (tokens, value)) = assignment(input, separator, &operation, strict_json_values)?;

    Ok((
        input,
        Jqesque::from_parts_unchecked(tokens, value, operation),
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
    strict_json_values: bool,
) -> Res<&'a str, (Vec<PathToken>, Option<Value>)> {
    let (input, tokens) = path(input, separator)?;

    let (input, value_opt) = match operation {
        Operation::Remove => (input, None),
        _ => {
            let (input, _) = char('=')(input)?;
            let (input, _) = opt(char(' ')).parse(input)?;
            let (input, value) = json_value(input, strict_json_values)?;
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

fn json_value(input: &str, strict_json_values: bool) -> Res<&str, Value> {
    map_res(is_not(""), move |s: &str| {
        if strict_json_values {
            serde_json::from_str::<Value>(s)
                .map_err(|e| format!("Invalid JSON value in strict mode: {e}"))
        } else {
            Ok(serde_json::from_str(s).unwrap_or(Value::String(s.to_string())))
        }
    })
    .parse(input)
}
