use std::borrow::BorrowMut;

use crate::types::PathToken;
use serde_json::{Map, Value};

/// Inserts a value into the JSON object at the specified path tokens.
///
/// # Arguments
///
/// * `json_obj` - The JSON object to insert into.
/// * `tokens` - The path tokens representing where to insert.
/// * `value` - The value to insert.
pub fn insert_value(json_obj: &mut Value, tokens: &[PathToken], value: &Value) {
    if tokens.is_empty() {
        *json_obj = value.clone();
        return;
    }

    match &tokens[0] {
        PathToken::Key(key) => {
            if !json_obj.is_object() {
                *json_obj = Value::Object(Map::new());
            }
            let entry = json_obj
                .as_object_mut()
                .unwrap()
                .entry(key.clone())
                .or_insert(Value::Null);
            insert_value(entry, &tokens[1..], value);
        }
        PathToken::Index(index) => {
            if !json_obj.is_array() {
                *json_obj = Value::Array(vec![]);
            }
            let array = json_obj.as_array_mut().unwrap();
            // Extend the array if necessary
            if *index >= array.len() {
                array.resize(*index + 1, Value::Null);
            }
            insert_value(&mut array[*index], &tokens[1..], value);
        }
    }
}

/// Merges two JSON values.
///
/// # Arguments
///
/// * `a` - The original JSON value.
/// * `b` - The new JSON value to merge in.
pub fn merge_json(a: &mut Value, b: &mut Value) {
    match (a.borrow_mut(), b) {
        (Value::Object(a_map), Value::Object(b_map)) => {
            for (k, v) in b_map.iter_mut() {
                merge_json(a_map.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (Value::Array(a_array), Value::Array(b_array)) => {
            for (i, v) in b_array.iter_mut().enumerate() {
                if i < a_array.len() {
                    merge_json(&mut a_array[i], v);
                } else {
                    a_array.push(v.take());
                }
            }
        }
        (_, b_value) => {
            *a = b_value.take();
        }
    }
}

#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use super::{insert_value, merge_json};
    use rstest::rstest;
    use serde_json::json;

    #[allow(unused_imports)]
    use crate::{Jqesque, JqesqueError, PathToken, Separator};

    #[allow(dead_code)]
    fn base_json() -> serde_json::Value {
        json!({
            "key": "value"
        })
    }

    #[rstest]
    #[case::new_keys(
        json!({"key2": "value2"}),
        json!({"key": "value", "key2": "value2"})
    )]
    #[case::nested_keys(
        json!({"parent": {"child": "value"}}),
        json!({"key": "value", "parent": {"child": "value"}})
    )]
    #[case::nested_array(json!({"array": [1]}), json!({"key": "value", "array": [1]}))]
    fn test_merge_json_ok(
        #[case] new_data: serde_json::Value,
        #[case] expected: serde_json::Value,
    ) {
        let mut json_obj = base_json();
        let mut new_data = new_data;
        merge_json(&mut json_obj, &mut new_data);

        assert_eq!(json_obj, expected);
    }

    #[rstest]
    #[case::empty_path(vec![], json!("value"), json!("value"))]
    #[case::single_key(vec!["key"], json!("value"), json!({"key": "value"}))]
    #[case::nested_keys(
        vec!["key2", "key3"],
        json!("value"),
        json!({"key2": {"key3": "value"}})
    )]
    fn test_insert_value_ok(
        #[case] tokens: Vec<&str>,
        #[case] value: serde_json::Value,
        #[case] expected: serde_json::Value,
    ) {
        let mut json_obj = serde_json::Value::Null;
        let tokens: Vec<_> = tokens
            .iter()
            .map(|s| s.to_string())
            .map(PathToken::Key)
            .collect();
        insert_value(&mut json_obj, &tokens, &value);

        assert_eq!(json_obj, expected);
    }

    #[rstest]
    #[case::negative_index("arr[-1]=value", Separator::Dot)]
    #[case::invalid_index("arr[invalid]=value", Separator::Dot)]
    #[case::missing_value("key=", Separator::Dot)]
    #[case::missing_key("=value", Separator::Dot)]
    #[case::missing_assignment("key", Separator::Dot)]
    #[case::illegal_operator("!key=value", Separator::Dot)]
    fn test_parse_input_err(#[case] input: &str, #[case] separator: Separator) {
        let result = Jqesque::from_str_with_separator(input, separator);
        assert!(
            matches!(result, Err(JqesqueError::NomError(_))),
            "{result:?}"
        );
    }
}
