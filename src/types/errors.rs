use std::fmt;

use serde_json::Value;
use thiserror::Error;

use super::Operation;

/// Resource whose configured or global ceiling was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LimitKind {
    InputBytes,
    PathDepth,
    PathBytes,
    ArrayIndex,
    ArraySlots,
    ValueDepth,
    DocumentDepth,
    ValueNodes,
    ValueBytes,
    BatchLength,
}

impl fmt::Display for LimitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InputBytes => "input bytes",
            Self::PathDepth => "path depth",
            Self::PathBytes => "path bytes",
            Self::ArrayIndex => "array index",
            Self::ArraySlots => "array slots",
            Self::ValueDepth => "value depth",
            Self::DocumentDepth => "document depth",
            Self::ValueNodes => "value nodes",
            Self::ValueBytes => "value bytes",
            Self::BatchLength => "batch length",
        })
    }
}

/// A syntax location: zero-based UTF-8 byte offset and one-based line/character column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    offset: usize,
    line: usize,
    column: usize,
}

impl SourceLocation {
    pub(crate) fn at(input: &str, offset: usize) -> Self {
        let prefix = &input[..offset];
        Self {
            offset,
            line: prefix.bytes().filter(|b| *b == b'\n').count() + 1,
            column: prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1,
        }
    }

    pub fn offset(self) -> usize {
        self.offset
    }
    pub fn line(self) -> usize {
        self.line
    }
    pub fn column(self) -> usize {
        self.column
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {} column {}", self.line, self.column)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SyntaxKind {
    ExpectedPath,
    ExpectedAssignment,
    ExpectedValue,
    InvalidArrayIndex,
    InvalidQuotedKey,
    UnknownOperation,
    TrailingInput,
}

impl fmt::Display for SyntaxKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ExpectedPath => "expected a path key or bracketed array index",
            Self::ExpectedAssignment => "expected '=' after the path",
            Self::ExpectedValue => "expected a value after '='",
            Self::InvalidArrayIndex => "expected a nonnegative array index followed by ']'",
            Self::InvalidQuotedKey => "expected a valid JSON string for the quoted key",
            Self::UnknownOperation => "unknown operation name",
            Self::TrailingInput => "unexpected input after the path",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathErrorKind {
    MissingPath,
    InvalidIndex,
    IndexOutOfBounds,
    TypeConflict,
    RootRemoval,
}

impl fmt::Display for PathErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Error, Debug, PartialEq)]
#[non_exhaustive]
pub enum JqesqueError {
    #[error("{kind} at {location}")]
    SyntaxError {
        kind: SyntaxKind,
        location: SourceLocation,
    },
    #[error("operation {0} requires a value")]
    MissingValueError(Operation),
    #[error("operation {0} does not accept a value")]
    UnexpectedValueError(Operation),
    #[error("{kind} at token {token} of JSON Pointer {path:?}")]
    PathError {
        kind: PathErrorKind,
        path: String,
        token: usize,
    },
    #[error("test failed: expected {expected} but found {actual}")]
    TestFailedError { expected: Value, actual: Value },
    #[error("invalid JSON value at {location}: {message}")]
    InvalidJsonValueError {
        location: SourceLocation,
        message: String,
    },
    #[error("limit exceeded for {kind}: limit={limit}, found={found}")]
    LimitExceededError {
        kind: LimitKind,
        limit: usize,
        found: usize,
    },
    #[error("operation {0} cannot be represented by this conversion")]
    UnsupportedConversion(Operation),
}

pub(crate) fn check_limit(kind: LimitKind, found: usize, limit: usize) -> Result<(), JqesqueError> {
    if found > limit {
        Err(JqesqueError::LimitExceededError { kind, limit, found })
    } else {
        Ok(())
    }
}
