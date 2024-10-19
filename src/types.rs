use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::manipulators::{insert_value, merge_json};
use crate::parse::parse_input;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Jqesque {
    // The path tokens representing the path to the value (the left-hand side of the assignment)
    pub tokens: Vec<PathToken>,
    // The value itself (the right-hand side of the assignment)
    pub value: Value,
}

impl FromStr for Jqesque {
    type Err = ParseError;

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
    /// ``````
    pub fn from_str_with_separator(input: &str, separator: Separator) -> Result<Self, ParseError> {
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
    /// assert_eq!(jqesque.value(), &serde_json::json!("hello"));
    /// ```
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Converts the parsed structure into a new JSON object.
    ///
    /// This function returns a new JSON object representing the parsed structure.
    ///
    /// For inserting or merging the parsed Jqesque structure into an existing JSON
    /// structure, it is recommended to use `insert_into` or `merge_into` instead.
    ///
    /// ## Returns
    ///
    /// Returns a `serde_json::Value` object.
    pub fn as_json(&self) -> Value {
        let mut json_obj = Value::Null;
        insert_value(&mut json_obj, &self.tokens, &self.value);
        json_obj
    }

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
    /// // Using insert_into will overwrite the existing "theme" object
    /// jqesque.insert_into(&mut json_obj);
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
    pub fn insert_into(&self, json: &mut Value) {
        insert_value(json, &self.tokens, &self.value);
    }

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
    /// let input = "settings.theme={\"color\":\"blue\",\"font\":\"Helvetica\"}";
    /// let separator = Separator::Dot;
    ///
    /// let jqesque = Jqesque::from_str_with_separator(input, separator).unwrap();
    /// // Using merge_into will merge the new "theme" object with the existing one
    /// jqesque.merge_into(&mut json_obj);
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
    pub fn merge_into(&self, json: &mut Value) {
        let mut temp_value = Value::Null;
        insert_value(&mut temp_value, &self.tokens, &self.value);
        merge_json(json, &mut temp_value);
    }
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

#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("Parsing error: {0}")]
    NomError(String),
}
