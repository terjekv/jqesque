use jqesque::{insert_value, merge_json, PathToken};
use serde_json::json;
use yare::parameterized;

fn base_json() -> serde_json::Value {
    json!({
        "key": "value"
    })
}

#[parameterized(
    new_keys = { json!({"key2": "value2"}), json!({"key": "value", "key2": "value2"}) },
    nested_keys = { json!({"parent": {"child": "value"}}), json!({"key": "value", "parent": {"child": "value"}}) },
    nested_array = { json!({"array": [1]}), json!({"key": "value", "array": [1]}) },
)]
fn test_merge_json_ok(new_data: serde_json::Value, expected: serde_json::Value) {
    let mut json_obj = base_json();
    let mut new_data = new_data;
    merge_json(&mut json_obj, &mut new_data);

    assert_eq!(json_obj, expected);
}

#[parameterized(
    empty_path = { vec![], json!("value"), json!("value") },
    single_key = { vec!["key"], json!("value"), json!({"key": "value"}) },
    nested_keys = { vec!["key2", "key3"], json!("value"), json!({"key2": {"key3": "value"}}) },
)]
fn test_insert_value_ok(tokens: Vec<&str>, value: serde_json::Value, expected: serde_json::Value) {
    let mut json_obj = serde_json::Value::Null;
    let tokens: Vec<_> = tokens
        .iter()
        .map(|s| s.to_string())
        .map(PathToken::Key)
        .collect();
    insert_value(&mut json_obj, &tokens, &value);

    assert_eq!(json_obj, expected);
}
