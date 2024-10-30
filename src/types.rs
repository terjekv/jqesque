use std::fmt::Display;
use std::str::FromStr;

use json_patch::{AddOperation, Patch, PatchOperation, RemoveOperation, ReplaceOperation};
use jsonptr::{Pointer, PointerBuf, Token};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

use crate::manipulators::{insert_value, merge_json};
use crate::parse::parse_input;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Jqesque {
    // The path tokens representing the path to the value (the left-hand side of the assignment)
    pub tokens: Vec<PathToken>,
    // The value itself (the right-hand side of the assignment)
    pub value: Option<Value>,
    // The operation to perform
    pub operation: Operation,
}

impl FromStr for Jqesque {
    type Err = JqesqueError;

    /// Parses an input string into a `Jqesque` structure using the default separator of `Separator::Dot`.
    ///
    /// ## Arguments
    ///
    /// * `input` - The input string to parse
    ///
    /// ## Returns
    ///
    /// Returns a `Jqesque` structure if successful, or a `ParseError` if parsing fails.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use jqesque::Jqesque;
    ///
    /// // Input string to parse
    /// let input = "foo.bar[0].baz=hello";
    /// let jqesque = input.parse::<Jqesque>().unwrap();
    /// // Without turbofish syntax:
    /// // let jqesque: Jqesque = input.parse().unwrap();
    /// ``````
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        parse_input(input, Separator::Dot)
    }
}

impl Jqesque {
    /// Parses an input string into a `Jqesque` structure using the specified separator.
    ///
    /// ## Arguments
    ///
    /// * `input` - The input string to parse
    /// * `separator` - The separator to use between keys (e.g., `Separator::Dot`, `Separator::Slash`, or `Separator::Custom(char)`)
    ///
    /// ## Input operators
    ///
    /// The input string can optionally start with an operator character to specify the operation to perform.
    ///
    /// * `>` - **Insert:** Inserts the value into the JSON object at the specified path, using a custom insert operation.
    /// * `~` - **Merge:** Merges the value into the JSON object at the specified path, using a custom merge operation.
    /// * `+` - **Add:** Adds the value to the JSON object at the specified path, using the JSON Patch `add` operation.
    /// * `-` - **Remove:** Removes the value from the JSON object at the specified path, using the JSON Patch `remove` operation.
    /// * `=` - **Replace:** Replaces the value in the JSON object at the specified path, using the JSON Patch `replace` operation.
    /// * `?` - **Test:** Tests the value in the JSON object at the specified path, using the JSON Patch `test` operation.
    ///
    /// If no operator is specified, the default operator is `Insert`. For details on each operation, see their respective
    /// fields in the `Operation` enum.
    ///
    /// ## Returns
    ///
    /// Returns a `Jqesque` structure if successful, or a `ParseError` if parsing fails.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use jqesque::{Jqesque, Separator};
    ///
    /// // Input string to parse
    /// let input = "foo.bar[0].baz=hello";
    /// let separator = Separator::Dot;
    /// let jqesque = Jqesque::from_str_with_separator(input, separator).unwrap();
    /// ```
    pub fn from_str_with_separator(
        input: &str,
        separator: Separator,
    ) -> Result<Self, JqesqueError> {
        parse_input(input, separator)
    }

    /// Returns the path tokens of the parsed structure.
    pub fn tokens(&self) -> &[PathToken] {
        &self.tokens
    }

    /// Returns the value from the parsed structure.
    ///
    /// This function returns a reference to the `serde_json::Value` object that was parsed.
    ///
    /// ## Returns
    ///
    /// Returns a reference to a `serde_json::Value` object.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use serde_json::Value;
    /// use jqesque::Jqesque;
    ///
    /// // Input string to parse
    /// let input = "foo.bar[0].baz=hello";
    /// let jqesque = input.parse::<Jqesque>().unwrap();
    ///
    /// match jqesque.value() {
    ///    Some(value) => {
    ///       assert_eq!(value, &serde_json::json!("hello"));
    ///   }
    ///   None => {
    ///     panic!("Expected a value, but found None");
    ///   }
    /// }
    /// ```
    pub fn value(&self) -> &Option<Value> {
        &self.value
    }

    /// Converts the parsed structure into a new JSON object.
    ///
    /// This function returns a new JSON object representing the parsed structure.
    ///
    /// ## Returns
    ///
    /// Returns a `serde_json::Value` object.
    pub fn as_json(&self) -> Value {
        match self.operation {
            Operation::Auto => {
                // For auto, return as an array of operations
                let mut json_obj = Value::Array(Vec::new());
                for op in &[Operation::Replace, Operation::Add, Operation::Insert] {
                    let mut jq = self.clone();
                    jq.operation = op.clone();
                    let op_json = jq.as_json();
                    json_obj.as_array_mut().unwrap().push(op_json);
                }
                json_obj
            }
            Operation::Add | Operation::Replace | Operation::Remove | Operation::Test => {
                let pointer_buf = self.tokens_to_pointer();
                let op_json = match self.operation {
                    Operation::Add | Operation::Replace | Operation::Test => json!({
                        "op": self.operation.to_string(),
                        "path": pointer_buf.to_string(),
                        "value": self.value.clone().unwrap_or(Value::Null)
                    }),
                    Operation::Remove => json!({
                        "op": self.operation.to_string(),
                        "path": pointer_buf.to_string()
                    }),
                    _ => unreachable!(),
                };
                json!([op_json]) // Return as an array of operations
            }
            Operation::Merge | Operation::Insert => {
                // For merge and insert, return the value to be merged or inserted
                let mut json_obj = Value::Null;
                insert_value(&mut json_obj, &self.tokens, &self.value);
                json_obj
            }
        }
    }

    /// Applies the parsed structure to a JSON object.
    ///
    /// This function applies the parsed structure to the provided JSON object, performing the operation specified
    /// during parsing. The operation is performed in-place, modifying the provided JSON object.
    ///
    /// ## Arguments
    ///
    /// * `json` - The JSON object to apply the operation to
    ///
    /// ## Returns
    ///
    /// Returns the operation that was performed or a JqesqueError if an error occurred.
    pub fn apply_to(&self, json: &mut Value) -> Result<Operation, JqesqueError> {
        match self.operation {
            Operation::Auto => {
                // Try Replace
                let mut jq_replace = self.clone();
                jq_replace.operation = Operation::Replace;
                if jq_replace.apply_to(json).is_ok() {
                    return Ok(Operation::Replace);
                }

                // Try Add
                let mut jq_add = self.clone();
                jq_add.operation = Operation::Add;
                if jq_add.apply_to(json).is_ok() {
                    return Ok(Operation::Add);
                }

                // Fallback to Insert
                let mut jq_insert = self.clone();
                jq_insert.operation = Operation::Insert;
                jq_insert.apply_to(json)
            }
            Operation::Add | Operation::Replace => {
                if let Some(ref value) = self.value {
                    let pointer_buf = self.tokens_to_pointer();

                    let patch_op = match self.operation {
                        Operation::Add => PatchOperation::Add(AddOperation {
                            path: pointer_buf,
                            value: value.clone(),
                        }),
                        Operation::Replace => PatchOperation::Replace(ReplaceOperation {
                            path: pointer_buf,
                            value: value.clone(),
                        }),
                        _ => unreachable!(),
                    };

                    let patch = Patch(vec![patch_op]);
                    json_patch::patch(json, &patch)
                        .map_err(|e| JqesqueError::PatchError(e.to_string()))?;
                    Ok(self.operation.clone())
                } else {
                    Err(JqesqueError::MissingValueError(self.operation.clone()))
                }
            }
            Operation::Remove => {
                let pointer_buf = self.tokens_to_pointer();

                let patch_op = PatchOperation::Remove(RemoveOperation { path: pointer_buf });
                let patch = Patch(vec![patch_op]);
                json_patch::patch(json, &patch)
                    .map_err(|e| JqesqueError::PatchError(e.to_string()))?;
                Ok(Operation::Remove)
            }
            Operation::Test => {
                if let Some(ref expected_value) = self.value {
                    let pointer_buf = self.tokens_to_pointer();
                    let pointer: &Pointer = &pointer_buf;

                    match pointer.resolve(json) {
                        Ok(actual_value) => {
                            if actual_value == expected_value {
                                Ok(Operation::Test)
                            } else {
                                Err(JqesqueError::TestFailedError {
                                    expected: expected_value.clone(),
                                    actual: actual_value.clone(),
                                })
                            }
                        }
                        Err(e) => Err(JqesqueError::InvalidPathError(e.to_string())),
                    }
                } else {
                    Err(JqesqueError::MissingValueError(self.operation.clone()))
                }
            }
            Operation::Merge => {
                // Assuming no errors occur during merge
                let mut temp_value = Value::Null;
                insert_value(&mut temp_value, &self.tokens, &self.value);
                merge_json(json, &mut temp_value);
                Ok(Operation::Merge)
            }
            Operation::Insert => {
                // Assuming no errors occur during insert
                insert_value(json, &self.tokens, &self.value);
                Ok(Operation::Insert)
            }
        }
    }

    /// Converts the path tokens to a JSON Pointer.
    ///
    /// This function converts the path tokens to a JSON Pointer, which is a string representation of the path.
    ///
    /// ## Returns
    ///
    /// Returns a `PointerBuf` object representing the path tokens.
    fn tokens_to_pointer(&self) -> PointerBuf {
        let tokens = self.tokens.iter().map(|token| match token {
            PathToken::Key(ref key) => Token::new(escape_json_pointer_segment(key)),
            PathToken::Index(idx) => Token::new(idx.to_string()),
        });

        PointerBuf::from_tokens(tokens)
    }
}

/// Helper function to escape JSON Pointer segments.
///
/// This is necessary to escape the characters '~' and '/' in JSON Pointer segments, as per
/// the JSON Pointer specification evalution:
/// https://datatracker.ietf.org/doc/html/rfc6901#section-4
///
/// ## Arguments
///
/// * `segment` - The segment to escape
///
/// ## Returns
///
/// Returns the escaped segment as a `String`.
fn escape_json_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PathToken {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone, Copy)]
pub enum Separator {
    Dot,
    Slash,
    Custom(char),
}

impl Separator {
    pub fn as_char(&self) -> char {
        match self {
            Separator::Dot => '.',
            Separator::Slash => '/',
            Separator::Custom(c) => *c,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Operation {
    /// **Inserts** the parsed structure into the provided JSON object.
    ///
    /// Inserts the value into the provided JSON object at the path found during parsing. If the
    /// path already exists in the JSON object, the existing value at that path will be **overwritten**,
    /// potentially replacing entire objects or arrays.
    ///
    /// Use this function when you want to set or replace a value exactly as specified, disregarding any
    /// existing data at that path.
    ///
    /// ## Arguments
    ///
    /// * `json` - The JSON object to insert into
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())`
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use serde_json::Value;
    /// use jqesque::{Jqesque, Separator};
    ///
    /// // Initial JSON object
    /// let mut json_obj = serde_json::json!({
    ///     "settings": {
    ///         "theme": {
    ///             "color": "red",
    ///             "font": "Arial",
    ///             "size": 12
    ///         }
    ///     }
    /// });
    ///
    /// // Input string to parse
    /// let input = "settings.theme={\"color\":\"blue\",\"font\":\"Helvetica\"}";
    /// let separator = Separator::Dot;
    ///
    /// let jqesque = Jqesque::from_str_with_separator(input, separator).unwrap();
    /// // Using apply_to with no explicit operator will use the operator "Insert" and this will
    /// // overwrite the existing "theme" object
    /// jqesque.apply_to(&mut json_obj);
    ///
    /// // The "theme" object is replaced entirely
    /// let expected = serde_json::json!({
    ///     "settings": {
    ///         "theme": {
    ///             "color": "blue",
    ///             "font": "Helvetica"
    ///         }
    ///     }
    /// });
    ///
    /// assert_eq!(json_obj, expected);
    ///
    /// // Note that the "size" key in the original "theme" object is removed
    /// ```
    ///
    /// In this example, `parse_and_insert` replaces the entire `"theme"` object with the new value,
    /// removing any existing keys not specified in the new value.    
    Insert,

    /// **Merges** the parsed structure into the JSON object.
    ///
    /// This function merges the value into the provided JSON object at path found during parsing.
    /// If the path already exists in the JSON object, the existing value at that path will be **merged**
    /// with the new value, combining objects and arrays rather than overwriting them.
    /// Existing keys not specified in the new value are preserved.
    ///
    /// Use this function when you want to update or extend the existing data without losing information,
    /// especially within nested objects or arrays.
    ///
    /// ## Arguments
    ///
    /// * `json` - The JSON object to merge into
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())`
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use serde_json::Value;
    /// use jqesque::{Jqesque, Separator};
    ///
    /// // Initial JSON object
    /// let mut json_obj = serde_json::json!({
    ///     "settings": {
    ///         "theme": {
    ///             "color": "red",
    ///             "font": "Arial",
    ///             "size": 12
    ///         }
    ///     }
    /// });
    ///
    /// // Input string to parse
    /// let input = "~settings.theme={\"color\":\"blue\",\"font\":\"Helvetica\"}";
    /// let separator = Separator::Dot;
    ///
    /// let jqesque = Jqesque::from_str_with_separator(input, separator).unwrap();
    /// // Prefixing the query with the merge operator (~) will merge the new
    /// // "theme" object with the existing one
    /// jqesque.apply_to(&mut json_obj);
    ///
    /// // The "theme" object is merged, updating existing keys and preserving others
    /// let expected = serde_json::json!({
    ///     "settings": {
    ///         "theme": {
    ///             "color": "blue",
    ///             "font": "Helvetica",
    ///             "size": 12
    ///         }
    ///     }
    /// });
    ///
    /// assert_eq!(json_obj, expected);
    ///
    /// // Note that the "size" key in the original "theme" object is preserved
    /// ```
    ///
    /// In this example, `parse_and_merge` updates the `"color"` and `"font"` keys within the `"theme"` object,
    /// while preserving the `"size"` key that was not specified in the new value.    
    Merge,
    Add,
    Remove,
    Replace,
    Test,

    /// **Auto** operation.
    ///
    /// The `Auto` operation will attempt the following operations in order:
    ///
    /// 1. **Replace**: If the path exists, replace the value.
    /// 2. **Add**: If the path does not exist, add the value.
    /// 3. **Insert**: If the path does not exist, insert the value.
    ///
    /// This operation is useful when you want to update a value if it exists, or add or insert it if it does not.
    Auto,
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op_str = match self {
            Operation::Insert => "insert",
            Operation::Merge => "merge",
            Operation::Add => "add",
            Operation::Remove => "remove",
            Operation::Replace => "replace",
            Operation::Test => "test",
            Operation::Auto => "auto",
        };
        write!(f, "{}", op_str)
    }
}

impl Operation {
    // Define the operator characters as associated constants
    const INSERT_OP: char = '>';
    const MERGE_OP: char = '~';
    const ADD_OP: char = '+';
    const REMOVE_OP: char = '-';
    const REPLACE_OP: char = '=';
    const TEST_OP: char = '?';

    // Get all valid operators
    pub fn operators() -> &'static [char] {
        &[
            Self::INSERT_OP,
            Self::MERGE_OP,
            Self::ADD_OP,
            Self::REMOVE_OP,
            Self::REPLACE_OP,
            Self::TEST_OP,
        ]
    }

    // Convert from operator character to Operation
    pub fn from_operator(op: char) -> Option<Self> {
        match op {
            Self::INSERT_OP => Some(Self::Insert),
            Self::MERGE_OP => Some(Self::Merge),
            Self::ADD_OP => Some(Self::Add),
            Self::REMOVE_OP => Some(Self::Remove),
            Self::REPLACE_OP => Some(Self::Replace),
            Self::TEST_OP => Some(Self::Test),
            _ => None,
        }
    }

    // Get the operator character for this operation
    pub fn to_operator(&self) -> Option<char> {
        match self {
            Self::Insert => Some(Self::INSERT_OP),
            Self::Merge => Some(Self::MERGE_OP),
            Self::Add => Some(Self::ADD_OP),
            Self::Remove => Some(Self::REMOVE_OP),
            Self::Replace => Some(Self::REPLACE_OP),
            Self::Test => Some(Self::TEST_OP),
            Self::Auto => None,
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum JqesqueError {
    #[error("Parsing error: {0}")]
    NomError(String),

    #[error("Operation {0} requires a value")]
    MissingValueError(Operation),

    #[error("JSON Patch error: {0}")]
    PatchError(String), // Store the error message as a string, json_patch::PatchError does not implement PartialEq

    #[error("Test failed: expected {expected} but found {actual}")]
    TestFailedError { expected: Value, actual: Value },

    #[error("Failed to access path: {0}")]
    InvalidPathError(String),
}
