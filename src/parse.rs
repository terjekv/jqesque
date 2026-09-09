use crate::types::{
    Jqesque, JqesqueError, Operation, ParseOptions, PathDepthLimit, PathToken, Separator,
};
use nom::{
    branch::alt,
    bytes::complete::{escaped, escaped_transform, is_not, take_while1},
    character::complete::{char, digit1, none_of, one_of},
    combinator::{all_consuming, map, map_res, opt},
    error::{ErrorKind, FromExternalError, ParseError},
    sequence::delimited,
    IResult, Parser,
};
use nom_language::error::VerboseError;
use serde_json::Value;

type Res<T, U> = IResult<T, U, ParserError<T>>;

#[derive(Debug)]
enum ParserError<I> {
    Syntax(VerboseError<I>),
    Limit(JqesqueError),
}

impl<I> ParseError<I> for ParserError<I> {
    fn from_error_kind(input: I, kind: ErrorKind) -> Self {
        Self::Syntax(VerboseError::from_error_kind(input, kind))
    }

    fn append(input: I, kind: ErrorKind, other: Self) -> Self {
        match other {
            Self::Syntax(error) => Self::Syntax(VerboseError::append(input, kind, error)),
            limit => limit,
        }
    }

    fn from_char(input: I, c: char) -> Self {
        Self::Syntax(VerboseError::from_char(input, c))
    }
}

impl<I, E> FromExternalError<I, E> for ParserError<I> {
    fn from_external_error(input: I, kind: ErrorKind, error: E) -> Self {
        Self::Syntax(VerboseError::from_external_error(input, kind, error))
    }
}

fn parse_error(error: nom::Err<ParserError<&str>>) -> JqesqueError {
    match error {
        nom::Err::Error(ParserError::Limit(error))
        | nom::Err::Failure(ParserError::Limit(error)) => error,
        nom::Err::Error(ParserError::Syntax(error)) => {
            JqesqueError::NomError(nom::Err::Error(error).to_string())
        }
        nom::Err::Failure(ParserError::Syntax(error)) => {
            JqesqueError::NomError(nom::Err::Failure(error).to_string())
        }
        nom::Err::Incomplete(needed) => {
            JqesqueError::NomError(nom::Err::<VerboseError<&str>>::Incomplete(needed).to_string())
        }
    }
}

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
    let depth_limit = PathDepthLimit::new(options.max_path_depth_limit());
    let (_, assignment) = all_consuming(|i| jqesque(i, sep_char, depth_limit))
        .parse(input)
        .map_err(parse_error)?;
    let value = assignment
        .value
        .map(|value| json_value(value, options.strict_json_values_enabled()))
        .transpose()?;
    let jqesque = Jqesque::new(assignment.tokens, value, assignment.operation)?;
    jqesque.validate_limits(depth_limit.get(), options.max_array_index_limit())?;
    Ok(jqesque)
}

fn jqesque(input: &str, separator: char, depth_limit: PathDepthLimit) -> Res<&str, Assignment<'_>> {
    let (input, operation) = opt(operation_prefix).parse(input)?;
    let operation = operation.unwrap_or(Operation::Auto);

    let (input, (tokens, value)) = assignment(input, separator, &operation, depth_limit)?;

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
    depth_limit: PathDepthLimit,
) -> Res<&'a str, (Vec<PathToken>, Option<&'a str>)> {
    let (input, tokens) = path(input, separator, depth_limit)?;

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

fn path(input: &str, separator: char, depth_limit: PathDepthLimit) -> Res<&str, Vec<PathToken>> {
    let mut tokens = Vec::new();
    let (mut input, ()) = path_segment(input, &mut tokens, depth_limit)?;

    while let Some(next) = input.strip_prefix(separator) {
        let previous_depth = tokens.len();
        match path_segment(next, &mut tokens, depth_limit) {
            Ok((remaining, ())) => input = remaining,
            Err(nom::Err::Error(_)) => {
                // A separator can also belong to the assignment (e.g. custom '=').
                tokens.truncate(previous_depth);
                break;
            }
            Err(error) => return Err(error),
        }
    }

    Ok((input, tokens))
}

fn path_segment<'a>(
    mut input: &'a str,
    tokens: &mut Vec<PathToken>,
    depth_limit: PathDepthLimit,
) -> Res<&'a str, ()> {
    let initial_depth = tokens.len();
    if input.starts_with('"') || input.chars().next().is_some_and(is_identifier_char) {
        if let Err(error) = depth_limit.check_next_token(tokens.len()) {
            // Recognize the excess key without allocating its decoded string. An invalid
            // key must still allow separator backtracking, e.g. custom '=' with value '"'.
            alt((
                delimited(
                    char('"'),
                    escaped(none_of("\\\""), '\\', one_of("\\\"nrt")),
                    char('"'),
                ),
                take_while1(is_identifier_char),
            ))
            .parse(input)?;
            return Err(nom::Err::Failure(ParserError::Limit(error)));
        }
        let (remaining, key) = alt((quoted_string, valid_identifier)).parse(input)?;
        tokens.push(PathToken::Key(key));
        input = remaining;
    }

    while input.starts_with('[') {
        match delimited(
            char('['),
            map_res(digit1, |s: &str| s.parse::<usize>()),
            char(']'),
        )
        .parse(input)
        {
            Ok((remaining, index)) => {
                depth_limit
                    .check_next_token(tokens.len())
                    .map_err(|error| nom::Err::Failure(ParserError::Limit(error)))?;
                tokens.push(PathToken::Index(index));
                input = remaining;
            }
            Err(nom::Err::Error(_)) => break,
            Err(error) => return Err(error),
        }
    }

    if tokens.len() == initial_depth {
        return Err(nom::Err::Error(ParserError::from_error_kind(
            input,
            ErrorKind::Alt,
        )));
    }
    Ok((input, ()))
}

fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn valid_identifier(input: &str) -> Res<&str, String> {
    map(take_while1(is_identifier_char), |s: &str| s.to_string()).parse(input)
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
