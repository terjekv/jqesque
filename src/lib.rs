//! # jqesque
//!
//! A Rust library to parse simplified JSON assignments in a jq-like syntax and convert them into JSON structures.
//!
//! Sometimes you want to express simplified JSON assignments as strings without writing the full JSON syntax. This library borrows syntax from [jq](https://jqlang.github.io/jq/) and JSONPath to create a simplified way to represent JSON assignments.
//!
//! ## Features
//!
//! - **Nested Objects:** Supports nested objects (e.g., `foo.bar.baz=true`).
//! - **Arrays with Indices:** Supports arrays with indices (e.g., `foo[0].bar=zoot`, where the index must be a positive number).
//! - **Boolean, Number, and Null Values:** Automatically parses values as booleans, numbers, or null if possible. By default, the value is a string unless serde can parse it as a boolean, number, or null.
//! - **Custom Separators:** Scopes can be separated by `Separator::Dot` (`.`), `Separator::Slash` (`/`), or `Separator::Custom(char)` (custom character).
//!
//! ## Syntax
//!
//! The syntax is inspired by [jq](https://jqlang.github.io/jq/) and [JSONPath](https://goessner.net/articles/JsonPath/) and is as follows:
//!
//! ```text
//! [<operation>]<path>=[<value>]
//! ```
//!
//! - `<operation>`: An optional operation to perform. Supported operations are Add (+), Replace (=), Remove (-), Test (?), Insert (>), and Merge (~).
//! - `<path>`: The path to the JSON key. The path can be nested and can include array indices. The path can be separated by a dot (`.`), a slash (`/`), or a custom character.
//! - `<value>`: A JSON value. Note that the Remove operation does not require a value.
//!
//! ### Operations
//!
//! Add, Remove, Replace, and Test operations are done as per the JSON Patch specification in [RFC6902](https://datatracker.ietf.org/doc/html/rfc6902/).
//!
//! - **Add (+):** Adds a new key-value pair to the JSON structure. If the key already exists, the operation fails. If the key is an array index, the operation appends the value to the array.
//! - **Remove (-):** Removes the key from the JSON structure.
//! - **Replace (=):** Replaces the value of an existing key. If the key does not exist, the operation fails.
//! - **Test (?):** Tests if the key-value pair exists in the JSON structure.
//! - **Insert (>):** Inserts a new key-value pair into the JSON structure. If the key already exists, the operation overwrites the value.
//! - **Merge (~):** Preforms a deep merge of the value into the existing JSON structure. null values are preserved in the existing structure. Note that this behavior **differs** from from JSON Merge Patch defined in [RFC7396](https://datatracker.ietf.org/doc/html/rfc7396).
//!
//! For more information, see the Operation enum itself.
//!
//! ### Paths
//!
//! Paths can be nested and can include array indices. The path can be separated by a dot (`.`), a slash (`/`), or a custom character.
//!
//! ### Values
//!
//! Values are parsed as by serde_json. The library will attempt to parse the value as a JSON value, defaulting to string.
//!
//! ## Examples
//!
//! ### Basic usage:
//!
//! ```rust
//! use jqesque::Jqesque;
//! use serde_json::json;
//!
//! let input = ">foo.bar[0].baz=hello";
//! let jqesque = input.parse::<Jqesque>().unwrap();
//! // Without using turbofish syntax:
//! // let jqesque: Jqesque = input.parse().unwrap();
//! // Alternatively, if you want to specify the separator:
//! // let jqesque = Jqesque::from_str_with_separator(input, Separator::Dot).unwrap();
//!
//! let json_output = jqesque.as_json();
//! assert_eq!(json_output, json!({
//!     "foo": {
//!         "bar": [
//!             {
//!                 "baz": "hello"
//!             }
//!         ]
//!     }
//! }));
//! ```
//!
//! ### Specifying the separator
//!
//! ```rust
//! use jqesque::{Jqesque, Separator};
//! use serde_json::json;
//!
//! let input = ">foo/bar[0]/baz=true";
//! let jqesque = Jqesque::from_str_with_separator(input, Separator::Slash).unwrap();
//! let json_output = jqesque.as_json();
//!
//! assert_eq!(json_output, json!({
//!     "foo": {
//!         "bar": [
//!             {
//!                 "baz": true
//!             }
//!         ]
//!     }
//! }));
//! ```
//!
//! ### Inserting into an existing JSON structure
//!
//! ```rust
//! use serde_json::json;
//! use jqesque::{Jqesque, Separator};
//!
//! let mut json_obj = json!({
//!     "settings": {
//!         "theme": {
//!             "color": "red",
//!             "font": "Arial",
//!             "size": 12
//!         }
//!     }
//! });
//!
//! let input = ">settings.theme={\"color\":\"blue\",\"font\":\"Helvetica\"}";
//! let jqesque = Jqesque::from_str_with_separator(input, Separator::Dot).unwrap();
//!
//! jqesque.apply_to(&mut json_obj);
//!
//! let expected = json!({
//!     "settings": {
//!         "theme": {
//!             "color": "blue",
//!             "font": "Helvetica"
//!         }
//!     }
//! });
//!
//! assert_eq!(json_obj, expected);
//! // Note that the "size" key in the original "theme" object is removed.
//! ```
//!
//! ### Merging into an existing JSON structure
//!
//! Prefix the query with `~` (The `merge` operator) to merge JSON objects.
//!
//! ### Do the right thing, hopefully...
//!
//! If no operator is specified, the library will first try to perform a `Replace` operation. If this fails,
//! it will attempt an `Add` operation. If this still fails, it will attempt an `Insert` operation.
//!
//! ```rust
//! use serde_json::json;
//! use jqesque::{Jqesque, Separator};
//!
//! let mut json_obj = json!({
//!     "settings": {
//!         "theme": {
//!             "color": "red",
//!             "font": "Arial",
//!             "size": 12
//!         }
//!     }
//! });
//!
//! let input = "~settings.theme={\"color\":\"blue\",\"font\":\"Helvetica\"}";
//! let jqesque = Jqesque::from_str_with_separator(input, Separator::Dot).unwrap();
//!
//! jqesque.apply_to(&mut json_obj);
//!
//! let expected = json!({
//!     "settings": {
//!         "theme": {
//!             "color": "blue",
//!             "font": "Helvetica",
//!             "size": 12
//!         }
//!     }
//! });
//!
//! assert_eq!(json_obj, expected);
//! // Note that the "size" key in the original "theme" object is preserved.
//! ```
//!
//! ## License
//!
//! See the [LICENSE](LICENSE) file for details.

mod manipulators;
mod parse;
mod types;

pub use types::{Jqesque, JqesqueError, Operation, PathToken, Separator};
