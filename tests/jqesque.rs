use jqesque::{Jqesque, JqesqueError, Operation, Separator};
use serde_json::json;
use yare::parameterized;

#[parameterized(
    simple_key = { "key=value", json!({"key": "value"}) } ,
    nested_keys = { "parent.child=value", json!({"parent": {"child": "value"}}) },
)]
fn test_using_from_str_with_default_separator(input: &str, expected: serde_json::Value) {
    let parsed = input.parse::<Jqesque>().expect("Failed to parse input");

    let mut json_obj = serde_json::Value::Null;
    parsed.apply_to(&mut json_obj).unwrap();

    assert_eq!(json_obj, expected);
}

// Tests for the `Insert` operation that should **succeed**.
#[allow(clippy::approx_constant)] // Since we use 3.14 as a test value
#[parameterized(
    simple_key = { ">key=value", Separator::Dot, json!({"key": "value"}) },
    nested_keys = { ">parent.child=value", Separator::Dot, json!({"parent": {"child": "value"}}) },
    array_index = { ">array[0]=1", Separator::Dot, json!({"array": [1]}) },
    nested_array = { ">array[0][1]=2", Separator::Dot, json!({"array": [[null, 2]]}) },
    custom_separator = { ">key1/key2=value", Separator::Slash, json!({"key1": {"key2": "value"}}) },
    quoted_key = { ">\"complex.key\"=123", Separator::Dot, json!({"complex.key": 123}) },
    bool_value = { ">flag=true", Separator::Dot, json!({"flag": true}) },
    null_value = { ">nothing=null", Separator::Dot, json!({"nothing": null}) },
    number_value = { ">number=42", Separator::Dot, json!({"number": 42}) },
    float_value = { ">pi=3.14", Separator::Dot, json!({"pi": 3.14}) },
    complex_path = { ">foo[1].bar[2]=value", Separator::Dot, json!({"foo": [null, {"bar": [null, null, "value"]}]}) },
    key_with_spaces = { ">\"key with spaces\"=value", Separator::Dot, json!({"key with spaces": "value"}) },
    array_of_objects = { ">items[0].name=Item1", Separator::Dot, json!({"items": [{"name": "Item1"}]}) },
    nested_objects = { ">obj.level1.level2=value", Separator::Dot, json!({"obj": {"level1": {"level2": "value"}}}) },
    empty_string_value = { ">empty=\"\"", Separator::Dot, json!({"empty": ""}) },
    special_chars_in_key = { ">\"key!@#$%^&*()\"=value", Separator::Dot, json!({"key!@#$%^&*()": "value"}) },
    unicode_key = { ">\"ключ\"=значение", Separator::Dot, json!({"ключ": "значение"}) },
    multiple_arrays = { ">a[0][1][2]=value", Separator::Dot, json!({
        "a": [
            [ // a[0]
                null,
                [ // a[0][1]
                    null,
                    null,
                    "value"
                ]
            ]
        ]
    }),
    },
    example_from_readme = { ">foo.bar[0].baz=true", Separator::Dot, json!({"foo": {"bar": [{"baz": true}]}}) },
    array_assignment_numbers = { ">arr=[1,2,3]", Separator::Dot, json!({"arr": [1, 2, 3]}) },
    array_assignment_text = { ">arr=[\"a\",\"b\",\"c\"]", Separator::Dot, json!({"arr": ["a", "b", "c"]}) },
    array_assignment_text_raw = { r#">arr=["a","b","c"]"#, Separator::Dot, json!({"arr": ["a", "b", "c"]}) },
)]
fn test_parse_input_insert_ok(input: &str, separator: Separator, expected: serde_json::Value) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");

    let mut json_obj = serde_json::Value::Null;
    parsed.apply_to(&mut json_obj).unwrap();

    assert_eq!(json_obj, expected);
}

// Tests for the `Add` operation that should **succeed**.
#[parameterized(
    add_to_existing_object = {
        "+parent.child.key=value", Separator::Dot,
        json!({
            "parent": {
                "child": {
                    "key": "value"
                }
            },
            "array": [1, 2, 3]
        })
    },
    add_to_existing_array = {
        "+array/1=42", Separator::Slash,
        json!({
            "parent": {
                "child": {}
            },
            "array": [1, 42, 2, 3]
        })
    },
    add_to_end_of_array = {
        "+array/-=99", Separator::Slash,
        json!({
            "parent": {
                "child": {}
            },
            "array": [1, 2, 3, 99]
        })
    },
)]
fn test_add_operation_success(input: &str, separator: Separator, expected: serde_json::Value) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation, Operation::Add);

    let mut json_obj = json!({
        "parent": {
            "child": {}
        },
        "array": [1, 2, 3]
    });

    parsed.apply_to(&mut json_obj).unwrap();

    assert_eq!(json_obj, expected);
}

// Tests for the `Add` operation that should **fail**.
#[parameterized(
    add_to_nonexistent_object = {
        "+nonexistent.key=value", Separator::Dot
    },
    add_to_nonexistent_array = {
        "+nonexistent_array/0=value", Separator::Slash
    },
    add_with_invalid_index = {
        "+array/10=value", Separator::Slash
    },
)]
fn test_add_operation_failure(input: &str, separator: Separator) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation, Operation::Add);

    let mut json_obj = json!({
        "parent": {
            "child": {}
        },
        "array": [1, 2, 3]
    });

    let result = parsed.apply_to(&mut json_obj);
    assert!(result.is_err(), "Expected error but operation succeeded");
}

// Tests for the `Replace` operation that should **succeed**.
#[parameterized(
    replace_existing_key = {
        "=parent.child.key=new_value", Separator::Dot,
        json!({
            "parent": {
                "child": {
                    "key": "new_value"
                }
            },
            "array": [1, 2, 3],
            "root": "root_value"
        })
    },
    replace_array_element = {
        "=array/1=42", Separator::Slash,
        json!({
            "parent": {
                "child": {
                    "key": "old_value"
                }
            },
            "array": [1, 42, 3],
            "root": "root_value"
        })
    },
    replace_root_key = {
        "=root=new_root_value", Separator::Dot,
        json!({
            "parent": {
                "child": {
                    "key": "old_value"
                }
            },
            "array": [1, 2, 3],
            "root": "new_root_value"
        })
    },
    replace_entire_object = {
        "=parent={\"new\": \"object\"}", Separator::Dot,
        json!({
            "parent": {
                "new": "object"
            },
            "array": [1, 2, 3],
            "root": "root_value"
        })
    },
    replace_with_null = {
        "=parent.child.key=null", Separator::Dot,
        json!({
            "parent": {
                "child": {
                    "key": null
                }
            },
            "array": [1, 2, 3],
            "root": "root_value"
        })
    },
)]
fn test_replace_operation_success(input: &str, separator: Separator, expected: serde_json::Value) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation, Operation::Replace);

    let mut json_obj = json!({
        "parent": {
            "child": {
                "key": "old_value"
            }
        },
        "array": [1, 2, 3],
        "root": "root_value"
    });

    parsed.apply_to(&mut json_obj).unwrap();

    assert_eq!(json_obj, expected);
}

// Tests for the `Replace` operation that should **fail**.
#[parameterized(
    replace_nonexistent_key = {
        "=parent.child.nonexistent_key=new_value", Separator::Dot
    },
    replace_nonexistent_array_element = {
        "=array/10=42", Separator::Slash
    },
    replace_nonexistent_root_key = {
        "=nonexistent_root_key=value", Separator::Dot
    },
    replace_in_non_object = {
        "=array/1/key=value", Separator::Slash      // Trying to access key in an array element
    },
)]
fn test_replace_operation_failure(input: &str, separator: Separator) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation, Operation::Replace);

    let mut json_obj = json!({
        "parent": {
            "child": {
                "key": "old_value"
            }
        },
        "array": [1, 2, 3],
        "root": "root_value"
    });

    let result = parsed.apply_to(&mut json_obj);
    assert!(result.is_err(), "Expected error but operation succeeded");
}

// Tests for the `Remove` operation that should **succeed**.
#[parameterized(
    remove_existing_key = {
        "-existing_key", Separator::Dot,
        json!({
            "array": [1, 2, 3],
            "nested": {
                "key": "value",
                "array": [1, 2, 3]
            }
        })
    },
    remove_nested_key = {
        "-nested.key", Separator::Dot,
        json!({
            "existing_key": "value",
            "array": [1, 2, 3],
            "nested": {
                "array": [1, 2, 3] 
            }
        })
    },
    remove_array_element = {
        "-array[1]", Separator::Dot,
        json!({
            "existing_key": "value",
            "array": [1, 3],
            "nested": {
                "key": "value",
                "array": [1, 2, 3] 
            }
        })
    },
    remove_entire_array = {
        "-array", Separator::Dot,
        json!({
            "existing_key": "value",
            "nested": {
                "key": "value",
                "array": [1, 2, 3]
            }
        })
    },
    remove_nested_array_element = {
        "-nested.array[0]", Separator::Dot,
        json!({
            "existing_key": "value",
            "array": [1, 2, 3],
            "nested": {
                "key": "value",
                "array": [2, 3]
            }
        })
    },
)]
fn test_remove_operation_success(input: &str, separator: Separator, expected: serde_json::Value) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation, Operation::Remove);

    let mut json_obj = json!({
        "existing_key": "value",
        "array": [1, 2, 3],
        "nested": {
            "key": "value",
            "array": [1, 2, 3]
        }
    });

    parsed.apply_to(&mut json_obj).unwrap();

    assert_eq!(json_obj, expected);
}

// Tests for the `Remove` operation that should **fail**.
#[parameterized(
    remove_nonexistent_key = {
        "-nonexistent_key", Separator::Dot
    },
    remove_nonexistent_nested_key = {
        "-nested.nonexistent_key", Separator::Dot
    },
    remove_nonexistent_array_element = {
        "-array[10]", Separator::Dot
    },
    remove_from_nonexistent_array = {
        "-nonexistent_array[0]", Separator::Dot
    },

)]
fn test_remove_operation_failure(input: &str, separator: Separator) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation, Operation::Remove);

    let mut json_obj = json!({
        "existing_key": "value",
        "array": [1, 2, 3],
        "nested": {
            "key": "value",
            "array": [1, 2, 3]
        }
    });

    let result = parsed.apply_to(&mut json_obj);
    assert!(result.is_err(), "Expected error but operation succeeded");
}

/// Tests for the `Auto` operation that should **succeed**.
#[parameterized(
    replace_existing_key = {
        "existing_key=new_value", Separator::Dot,
        json!({
            "existing_key": "new_value",
            "array": [1, 2, 3]
        }),
        Operation::Replace
    },
    add_new_key = {
        "new_key=new_value", Separator::Dot,
        json!({
            "existing_key": "old_value",
            "array": [1, 2, 3],
            "new_key": "new_value"
        }),
        Operation::Add
    },
    insert_new_nested_key = {
        "parent.new_child=new_value", Separator::Dot,
        json!({
            "existing_key": "old_value",
            "array": [1, 2, 3],
            "parent": {
                "new_child": "new_value"
            }
        }),
        Operation::Insert
    },
    replace_array_element = {
        "array[1]=42", Separator::Dot,
        json!({
            "existing_key": "old_value",
            "array": [1, 42, 3]
        }),
        Operation::Replace

    },
    add_to_array_end = {
        "array[3]=4", Separator::Dot,
        json!({
            "existing_key": "old_value",
            "array": [1, 2, 3, 4]
        }),
        Operation::Add
    },
    insert_new_array = {
        "new_array[0]=1", Separator::Dot,
        json!({
            "existing_key": "old_value",
            "array": [1, 2, 3],
            "new_array": [1]
        }),
        Operation::Insert
    },
)]
fn test_auto_operation(
    input: &str,
    separator: Separator,
    expected: serde_json::Value,
    epxected_operation: Operation,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation, Operation::Auto);

    let mut json_obj = json!({
        "existing_key": "old_value",
        "array": [1, 2, 3]
    });

    let operation = parsed.apply_to(&mut json_obj).unwrap();
    assert_eq!(operation, epxected_operation);

    assert_eq!(json_obj, expected);
}

/// Tests for patch errors.
#[parameterized(
        remove_nonexistent_key = { "-nonexistent", Separator::Dot },
        replace_nonexistent_key = { "=nonexistent=value", Separator::Dot },
        add_invalid_index = { "+array[10]=value", Separator::Dot },
    )]
fn test_patch_errors(input: &str, separator: Separator) {
    let parsed = Jqesque::from_str_with_separator(input, separator).unwrap();
    let mut json_obj = json!({ "array": [1, 2, 3] });
    let result = parsed.apply_to(&mut json_obj);
    assert!(
        result.is_err(),
        "Expected PatchError but operation succeeded"
    );
    match result {
        Err(JqesqueError::PatchError(_)) => (),
        Err(e) => panic!("Expected PatchError, got {:?}", e),
        _ => panic!("Expected error but operation succeeded"),
    }
}

/// Tests for the `Test` operation that should **succeed**.
#[parameterized(
    test_existing_key = { "?key=value", Separator::Dot, json!({ "key": "value" }) },
    test_nested_key = { "?parent.child=value", Separator::Dot, json!({ "parent": { "child": "value" } }) },
    test_array_element = { "?array[0]=1", Separator::Dot, json!({ "array": [1, 2, 3] }) },
    test_nested_array_element = { "?array[0][1]=2", Separator::Dot, json!({ "array": [[null, 2]] }) },
)]
fn test_test_operation_success(input: &str, separator: Separator, initial_json: serde_json::Value) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation, Operation::Test);

    let mut json_obj = initial_json;
    assert!(parsed.apply_to(&mut json_obj).is_ok());
}

/// Tests for test failed errors.
#[parameterized(
        test_value_mismatch = { "?key=expected_value", Separator::Dot, json!({ "key": "actual_value" }) },
        test_nonexistent_key = { "?nonexistent=value", Separator::Dot, json!({ "key": "value" }) },
    )]
fn test_test_failed_errors(input: &str, separator: Separator, initial_json: serde_json::Value) {
    let parsed = Jqesque::from_str_with_separator(input, separator).unwrap();
    let mut json_obj = initial_json;
    let result = parsed.apply_to(&mut json_obj);
    assert!(
        result.is_err(),
        "Expected TestFailedError but operation succeeded"
    );
    match result {
        Err(JqesqueError::TestFailedError { expected, actual }) => {
            println!(
                "Test failed as expected. Expected: {}, Actual: {}",
                expected, actual
            );
        }
        Err(JqesqueError::InvalidPathError(_)) => {
            // This can happen if the path doesn't exist
        }
        Err(e) => panic!("Expected TestFailedError, got {:?}", e),
        _ => panic!("Expected error but operation succeeded"),
    }
}

/// Tests for invalid path errors.
#[parameterized(
        invalid_path_syntax = { "+key..subkey=value", Separator::Dot },
        invalid_array_index = { "+array[-1]=value", Separator::Dot },
        invalid_escape_sequence = { "+key\\subkey=value", Separator::Dot },
    )]
fn test_invalid_path_errors(input: &str, separator: Separator) {
    let result = Jqesque::from_str_with_separator(input, separator);
    assert!(
        result.is_err(),
        "Expected parsing error due to invalid path but parsing succeeded"
    );
    match result {
        Err(JqesqueError::NomError(_)) => (),
        Err(e) => panic!("Expected NomError, got {:?}", e),
        _ => panic!("Expected error but parsing succeeded"),
    }
}

/// Tests for as_json method.
#[parameterized(
    simple_key = { ">key=value", json!({"key": "value"}) } ,
    nested_keys = { ">parent.child=value", json!({"parent": {"child": "value"}}) },
)]
fn test_as_json(input: &str, expected: serde_json::Value) {
    let json_obj = input
        .parse::<Jqesque>()
        .expect("Failed to parse input")
        .as_json();

    assert_eq!(json_obj, expected);
}
