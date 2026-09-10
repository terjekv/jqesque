use std::{fmt, str::FromStr};

use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};

use crate::manipulators::{
    apply_at_path, check_patch_path, numeric_json_eq, path_array_growth, prepare_patch, resolve,
};
use crate::parse::parse_input_with_options;

mod batch;
mod errors;
mod options;
mod path;
mod serde_support;
mod value;
use value::check_clone;
pub(crate) use value::discard;
pub(crate) use value::ValidatedValue;

pub use batch::{Batch, BatchError};
pub(crate) use errors::check_limit;
pub use errors::{JqesqueError, LimitKind, PathErrorKind, SourceLocation, SyntaxKind};
pub use options::*;
pub(crate) use path::PathBuilder;
pub use path::{Path, PathToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Separator {
    Dot,
    Slash,
    Custom(char),
}

impl Separator {
    pub fn as_char(&self) -> char {
        match self {
            Self::Dot => '.',
            Self::Slash => '/',
            Self::Custom(c) => *c,
        }
    }
}

/// Assignment behavior. Full names are case-sensitive and followed by whitespace.
/// See the crate documentation for the complete operation table and Auto examples.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Operation {
    /// Create missing containers and overwrite the selected value (`>` / `insert`).
    Insert,
    /// Deep merge objects and arrays by index; null is a value (`~` / `merge`).
    Merge,
    /// Apply RFC 7396 to the selected value (`merge-patch`): delete null object members,
    /// replace arrays wholesale. Selecting a nested value is a jqesque extension.
    MergePatch,
    /// RFC 6902 add: replace an object member or insert into an existing array (`+` / `add`).
    Add,
    /// Remove an existing object member or array element (`-` / `remove`).
    Remove,
    /// Replace an existing value (`=` / `replace`).
    Replace,
    /// Compare an existing value using RFC 6902 equality (`?` / `test`).
    Test,
    /// Default: try Replace, then Add, then Insert. Never performs a merge.
    Auto,
}

impl Operation {
    pub fn operators() -> &'static [char] {
        &['>', '~', '+', '-', '=', '?']
    }
    pub fn from_operator(op: char) -> Option<Self> {
        Some(match op {
            '>' => Self::Insert,
            '~' => Self::Merge,
            '+' => Self::Add,
            '-' => Self::Remove,
            '=' => Self::Replace,
            '?' => Self::Test,
            _ => return None,
        })
    }
    pub fn to_operator(&self) -> Option<char> {
        match self {
            Self::Insert => Some('>'),
            Self::Merge => Some('~'),
            Self::Add => Some('+'),
            Self::Remove => Some('-'),
            Self::Replace => Some('='),
            Self::Test => Some('?'),
            Self::Auto | Self::MergePatch => None,
        }
    }
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "insert" => Self::Insert,
            "merge" => Self::Merge,
            "merge-patch" => Self::MergePatch,
            "add" => Self::Add,
            "remove" => Self::Remove,
            "replace" => Self::Replace,
            "test" => Self::Test,
            "auto" => Self::Auto,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Insert => "insert",
            Self::Merge => "merge",
            Self::MergePatch => "merge-patch",
            Self::Add => "add",
            Self::Remove => "remove",
            Self::Replace => "replace",
            Self::Test => "test",
            Self::Auto => "auto",
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ValueOperation {
    Insert,
    Merge,
    MergePatch,
    Add,
    Replace,
    Test,
    Auto,
}

impl ValueOperation {
    fn operation(&self) -> Operation {
        match self {
            Self::Insert => Operation::Insert,
            Self::Merge => Operation::Merge,
            Self::MergePatch => Operation::MergePatch,
            Self::Add => Operation::Add,
            Self::Replace => Operation::Replace,
            Self::Test => Operation::Test,
            Self::Auto => Operation::Auto,
        }
    }
}

impl TryFrom<Operation> for ValueOperation {
    type Error = JqesqueError;
    fn try_from(operation: Operation) -> Result<Self, Self::Error> {
        Ok(match operation {
            Operation::Insert => Self::Insert,
            Operation::Merge => Self::Merge,
            Operation::MergePatch => Self::MergePatch,
            Operation::Add => Self::Add,
            Operation::Replace => Self::Replace,
            Operation::Test => Self::Test,
            Operation::Auto => Self::Auto,
            Operation::Remove => return Err(JqesqueError::UnexpectedValueError(operation)),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Action {
    Remove,
    WithValue {
        operation: ValueOperation,
        value: ValidatedValue,
    },
}

/// A validated assignment. Construction and deserialization enforce the same invariants.
#[derive(Debug, Clone, PartialEq)]
pub struct Jqesque {
    path: Path,
    action: Action,
}

impl Serialize for Jqesque {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state =
            serializer.serialize_struct("Jqesque", if self.value().is_some() { 3 } else { 2 })?;
        state.serialize_field("tokens", self.tokens())?;
        if let Some(value) = self.value() {
            state.serialize_field("value", value)?;
        }
        state.serialize_field("operation", &self.operation())?;
        state.end()
    }
}

fn present_value<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<ValidatedValue>, D::Error> {
    ValidatedValue::deserialize(deserializer).map(Some)
}

impl<'de> Deserialize<'de> for Jqesque {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Fields {
            tokens: Path,
            #[serde(default, deserialize_with = "present_value")]
            value: Option<ValidatedValue>,
            operation: Operation,
        }
        let fields = Fields::deserialize(deserializer)?;
        // Accept the legacy Remove encoding, which always emitted value:null.
        let value = if fields.operation == Operation::Remove
            && fields
                .value
                .as_ref()
                .is_some_and(|value| value.value.is_null())
        {
            None
        } else {
            fields.value
        };
        Self::from_validated(fields.tokens, value, fields.operation)
            .map_err(serde::de::Error::custom)
    }
}

impl FromStr for Jqesque {
    type Err = JqesqueError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::from_str_with_options(input, ParseOptions::default())
    }
}

impl Jqesque {
    pub fn new(
        tokens: Vec<PathToken>,
        value: Option<Value>,
        operation: Operation,
    ) -> Result<Self, JqesqueError> {
        match Path::new(tokens) {
            Ok(path) => Self::from_path(path, value, operation),
            Err(error) => {
                if let Some(value) = value {
                    discard(value);
                }
                Err(error)
            }
        }
    }

    /// Reuse a validated path without reconstructing its proof from raw tokens.
    pub fn from_path(
        path: Path,
        value: Option<Value>,
        operation: Operation,
    ) -> Result<Self, JqesqueError> {
        Self::from_validated(path, value.map(ValidatedValue::new).transpose()?, operation)
    }

    pub(crate) fn from_validated(
        path: Path,
        value: Option<ValidatedValue>,
        operation: Operation,
    ) -> Result<Self, JqesqueError> {
        check_limit(
            LimitKind::ValueBytes,
            path.key_bytes()
                .saturating_add(value.as_ref().map_or(0, |value| value.bytes)),
            DEFAULT_MAX_INPUT_BYTES,
        )?;
        let action = match (operation, value) {
            (Operation::Remove, None) => Action::Remove,
            (Operation::Remove, Some(_)) => {
                return Err(JqesqueError::UnexpectedValueError(operation))
            }
            (_, None) => return Err(JqesqueError::MissingValueError(operation)),
            (operation, Some(value)) => Action::WithValue {
                operation: operation.try_into()?,
                value,
            },
        };
        Ok(Self { path, action })
    }

    pub fn from_str_with_separator(
        input: &str,
        separator: Separator,
    ) -> Result<Self, JqesqueError> {
        Self::from_str_with_options(input, ParseOptions::new(separator))
    }
    pub fn from_str_with_options(input: &str, options: ParseOptions) -> Result<Self, JqesqueError> {
        parse_input_with_options(input, options)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn tokens(&self) -> &[PathToken] {
        self.path.tokens()
    }
    pub fn value(&self) -> Option<&Value> {
        match &self.action {
            Action::Remove => None,
            Action::WithValue { value, .. } => Some(&value.value),
        }
    }
    pub fn operation(&self) -> Operation {
        match &self.action {
            Action::Remove => Operation::Remove,
            Action::WithValue { operation, .. } => operation.operation(),
        }
    }

    /// Materialize an Insert or deep Merge assignment as a document fragment.
    /// This fragment is data, not an RFC 7396 patch. Applying it with an unrelated merge
    /// implementation need not reproduce `apply_to`, particularly for indexed paths.
    pub fn to_document(&self) -> Result<Value, JqesqueError> {
        if !matches!(self.operation(), Operation::Insert | Operation::Merge) {
            return Err(JqesqueError::UnsupportedConversion(self.operation()));
        }
        let mut value = Value::Null;
        let mut budget = ApplyBudget::new(ApplyOptions::default());
        self.apply_operation(&mut value, Operation::Insert, &mut budget)?;
        Ok(value)
    }

    /// Export Add, Remove, Replace or Test as an RFC 6902 patch array.
    /// Auto requires a target document and has no target-independent patch representation.
    pub fn to_json_patch(&self) -> Result<Value, JqesqueError> {
        let operation = self.operation();
        if !matches!(
            operation,
            Operation::Add | Operation::Remove | Operation::Replace | Operation::Test
        ) {
            return Err(JqesqueError::UnsupportedConversion(operation));
        }
        let mut patch = json!({"op": operation.name(), "path": self.path.to_json_pointer()});
        if let Some(value) = self.value() {
            patch["value"] = value.clone();
        }
        Ok(json!([patch]))
    }

    /// Apply one assignment. Errors leave the document unchanged.
    /// Returns the concrete operation chosen, including Auto's selected fallback.
    pub fn apply_to(&self, document: &mut Value) -> Result<Operation, JqesqueError> {
        self.apply_to_with_options(document, ApplyOptions::default())
    }

    pub fn apply_to_with_options(
        &self,
        document: &mut Value,
        options: ApplyOptions,
    ) -> Result<Operation, JqesqueError> {
        self.apply_with_budget(document, &mut ApplyBudget::new(options))
    }

    pub(crate) fn apply_with_budget(
        &self,
        document: &mut Value,
        budget: &mut ApplyBudget,
    ) -> Result<Operation, JqesqueError> {
        let operation = if self.operation() == Operation::Auto {
            if check_patch_path(document, &self.path, Operation::Replace).is_ok() {
                Operation::Replace
            } else if check_patch_path(document, &self.path, Operation::Add).is_ok() {
                Operation::Add
            } else {
                Operation::Insert
            }
        } else {
            self.operation()
        };
        self.apply_operation(document, operation, budget)?;
        Ok(operation)
    }

    fn apply_operation(
        &self,
        document: &mut Value,
        operation: Operation,
        budget: &mut ApplyBudget,
    ) -> Result<(), JqesqueError> {
        let value = self.value().unwrap_or(&Value::Null);
        if operation == Operation::Test {
            let actual = resolve(document, &self.path)?;
            return if numeric_json_eq(actual, value) {
                Ok(())
            } else {
                check_clone(actual)?;
                Err(JqesqueError::TestFailedError {
                    expected: value.clone(),
                    actual: actual.clone(),
                })
            };
        }
        let payload_slots = match &self.action {
            Action::Remove => 0,
            Action::WithValue { value, .. } => value.array_slots,
        };
        if matches!(
            operation,
            Operation::Insert | Operation::Merge | Operation::MergePatch
        ) {
            let growth = path_array_growth(document, &self.path);
            budget.spend(growth.saturating_add(payload_slots))?;
            apply_at_path(document, &self.path, value, operation);
            return Ok(());
        }
        let prepared = prepare_patch(document, &self.path, operation)?;
        budget.spend(prepared.array_slots().saturating_add(payload_slots))?;
        prepared.apply(value);
        Ok(())
    }

    pub(super) fn payload_nodes(&self) -> usize {
        match &self.action {
            Action::Remove => 0,
            Action::WithValue { value, .. } => value.nodes,
        }
    }

    pub(super) fn payload_bytes(&self) -> usize {
        self.path.key_bytes().saturating_add(match &self.action {
            Action::Remove => 0,
            Action::WithValue { value, .. } => value.bytes,
        })
    }

    /// Check tighter path limits for a caller's policy; global facts remain established by `Path`.
    pub fn validate_limits(
        &self,
        max_path_depth: usize,
        max_array_index: usize,
    ) -> Result<(), JqesqueError> {
        check_limit(LimitKind::PathDepth, self.tokens().len(), max_path_depth)?;
        for token in self.tokens() {
            if let PathToken::Index(index) = token {
                check_limit(LimitKind::ArrayIndex, *index, max_array_index)?;
            }
        }
        Ok(())
    }
}

pub(crate) struct ApplyBudget {
    remaining: usize,
    limit: usize,
}
impl ApplyBudget {
    fn new(options: ApplyOptions) -> Self {
        Self {
            remaining: options.max_array_slots_limit(),
            limit: options.max_array_slots_limit(),
        }
    }
    fn spend(&mut self, slots: usize) -> Result<(), JqesqueError> {
        check_limit(
            LimitKind::ArraySlots,
            self.limit
                .saturating_sub(self.remaining)
                .saturating_add(slots),
            self.limit,
        )?;
        self.remaining -= slots;
        Ok(())
    }
}
