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
