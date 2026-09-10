use serde::de::IgnoredAny;
use serde_json::Value;

use crate::types::{
    check_limit, Jqesque, JqesqueError, LimitKind, Operation, ParseOptions, Path, PathBuilder,
    PathToken, SourceLocation, SyntaxKind, ValidatedValue,
};

fn syntax(input: &str, offset: usize, kind: SyntaxKind) -> JqesqueError {
    JqesqueError::SyntaxError {
        kind,
        location: SourceLocation::at(input, offset),
    }
}

fn identifier(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

/// Return the byte length of a JSON quoted key without allocating its decoded form.
fn quoted_len(input: &str) -> Option<usize> {
    let mut escaped = false;
    for (index, byte) in input.bytes().enumerate().skip(1) {
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            let length = index + 1;
            return serde_json::from_str::<IgnoredAny>(&input[..length])
                .ok()
                .map(|_| length);
        }
    }
    None
}

pub(crate) fn parse_input_with_options(
    input: &str,
    options: ParseOptions,
) -> Result<Jqesque, JqesqueError> {
    check_limit(
        LimitKind::InputBytes,
        input.len(),
        options.max_input_bytes_limit(),
    )?;
    let (operation, mut offset) = operation(input, options)?;
    let mut builder = PathBuilder::new(options);
    let path = if input[offset..].starts_with('.')
        && matches!(input.as_bytes().get(offset + 1), None | Some(b'='))
    {
        offset += 1;
        Path::root()
    } else {
        segment(input, &mut offset, &mut builder, false)?;
        let separator = options.separator().as_char();
        while input[offset..].starts_with(separator) {
            let mut next = offset + separator.len_utf8();
            // Preserve the legacy custom '=' separator's assignment-delimiter backtracking.
            if separator == '=' && !segment_starts(&input[next..]) {
                break;
            }
            let before = builder.len();
            segment(input, &mut next, &mut builder, separator == '=')?;
            if builder.len() == before {
                break;
            }
            offset = next;
        }
        builder.finish()
    };
    let value = if operation == Operation::Remove {
        if offset != input.len() {
            return Err(syntax(input, offset, SyntaxKind::TrailingInput));
        }
        None
    } else {
        if input.as_bytes().get(offset) != Some(&b'=') {
            return Err(syntax(input, offset, SyntaxKind::ExpectedAssignment));
        }
        offset += 1;
        // Keep permissive parsing's historical removal of one optional leading space.
        if input.as_bytes().get(offset) == Some(&b' ') {
            offset += 1;
        }
        if offset == input.len() {
            return Err(syntax(input, offset, SyntaxKind::ExpectedValue));
        }
        Some(json_value(
            input,
            offset,
            options.strict_json_values_enabled(),
        )?)
    };
    Jqesque::from_validated(path, value, operation)
}

fn operation(input: &str, options: ParseOptions) -> Result<(Operation, usize), JqesqueError> {
    if let Some(op) = input.chars().next().and_then(Operation::from_operator) {
        return Ok((op, 1));
    }
    let name_len = input
        .chars()
        .take_while(|c| identifier(*c))
        .map(char::len_utf8)
        .sum::<usize>();
    if name_len > 0
        && input[name_len..].starts_with([' ', '\t'])
        && !matches!(options.separator().as_char(), ' ' | '\t')
    {
        let operation = Operation::from_name(&input[..name_len])
            .ok_or_else(|| syntax(input, 0, SyntaxKind::UnknownOperation))?;
        let offset = input.len() - input[name_len..].trim_start_matches([' ', '\t']).len();
        Ok((operation, offset))
    } else {
        Ok((Operation::Auto, 0))
    }
}

fn segment_starts(input: &str) -> bool {
    if input.starts_with('"') {
        quoted_len(input).is_some()
    } else if let Some(rest) = input.strip_prefix('[') {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        digits > 0 && rest.as_bytes().get(digits) == Some(&b']')
    } else {
        input.chars().next().is_some_and(identifier)
    }
}

fn segment(
    input: &str,
    offset: &mut usize,
    builder: &mut PathBuilder,
    backtrack: bool,
) -> Result<(), JqesqueError> {
    let before = builder.len();
    let rest = &input[*offset..];
    if rest.starts_with('"') {
        let Some(length) = quoted_len(rest) else {
            if backtrack {
                return Ok(());
            }
            return Err(syntax(input, *offset, SyntaxKind::InvalidQuotedKey));
        };
        builder.check_next()?;
        let key = serde_json::from_str::<String>(&rest[..length])
            .map_err(|_| syntax(input, *offset, SyntaxKind::InvalidQuotedKey))?;
        builder.push(PathToken::Key(key))?;
        *offset += length;
    } else {
        let length = rest
            .chars()
            .take_while(|c| identifier(*c))
            .map(char::len_utf8)
            .sum::<usize>();
        if length > 0 {
            builder.check_next()?;
            builder.push(PathToken::Key(rest[..length].to_owned()))?;
            *offset += length;
        }
    }
    while input[*offset..].starts_with('[') {
        let start = *offset + 1;
        let length = input[start..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        if length == 0 || input.as_bytes().get(start + length) != Some(&b']') {
            if backtrack && builder.len() == before {
                return Ok(());
            }
            return Err(syntax(input, start, SyntaxKind::InvalidArrayIndex));
        }
        let index = input[start..start + length]
            .parse::<usize>()
            .map_err(|_| syntax(input, start, SyntaxKind::InvalidArrayIndex))?;
        builder.push(PathToken::Index(index))?;
        *offset = start + length + 1;
    }
    if builder.len() == before {
        return Err(syntax(input, *offset, SyntaxKind::ExpectedPath));
    }
    Ok(())
}

fn json_value(input: &str, offset: usize, strict: bool) -> Result<ValidatedValue, JqesqueError> {
    let raw = &input[offset..];
    match serde_json::from_str(raw) {
        Ok(value) => Ok(value),
        Err(error) if strict => {
            // serde_json columns count bytes. Convert to a UTF-8 character location in the whole assignment.
            let line_start = raw
                .split_inclusive('\n')
                .take(error.line().saturating_sub(1))
                .map(str::len)
                .sum::<usize>();
            let mut relative = if error.is_eof() {
                raw.len()
            } else {
                (line_start + error.column().saturating_sub(1)).min(raw.len())
            };
            while !raw.is_char_boundary(relative) {
                relative -= 1;
            }
            Err(JqesqueError::InvalidJsonValueError {
                location: SourceLocation::at(input, offset + relative),
                message: error.to_string(),
            })
        }
        Err(_) => ValidatedValue::new(Value::String(raw.to_owned())),
    }
}
