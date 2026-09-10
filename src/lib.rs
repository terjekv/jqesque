#![doc = include_str!("../README.md")]

mod manipulators;
mod parse;
mod types;

pub use types::{
    ApplyOptions, Batch, BatchError, Jqesque, JqesqueError, LimitKind, Operation, ParseOptions,
    Path, PathErrorKind, PathToken, Separator, SourceLocation, SyntaxKind, DEFAULT_MAX_ARRAY_INDEX,
    DEFAULT_MAX_ARRAY_SLOTS, DEFAULT_MAX_BATCH_LENGTH, DEFAULT_MAX_INPUT_BYTES,
    DEFAULT_MAX_PATH_DEPTH, DEFAULT_MAX_VALUE_DEPTH, DEFAULT_MAX_VALUE_NODES,
};
