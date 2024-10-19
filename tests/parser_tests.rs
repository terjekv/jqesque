use jqesque::{insert_value, Jqesque, ParseError, Separator};
use serde_json::json;
use yare::parameterized;

#[allow(clippy::approx_constant)] // Since we use 3.14 as a test value
#[parameterized(
    simple_key = { "key=value", Separator::Dot, json!({"key": "value"}) },
    nested_keys = { "parent.child=value", Separator::Dot, json!({"parent": {"child": "value"}}) },
    array_index = { "array[0]=1", Separator::Dot, json!({"array": [1]}) },
    nested_array = { "array[0][1]=2", Separator::Dot, json!({"array": [[null, 2]]}) },
    custom_separator = { "key1/key2=value", Separator::Slash, json!({"key1": {"key2": "value"}}) },
    quoted_key = { "\"complex.key\"=123", Separator::Dot, json!({"complex.key": 123}) },
    bool_value = { "flag=true", Separator::Dot, json!({"flag": true}) },
    null_value = { "nothing=null", Separator::Dot, json!({"nothing": null}) },
    number_value = { "number=42", Separator::Dot, json!({"number": 42}) },
    float_value = { "pi=3.14", Separator::Dot, json!({"pi": 3.14}) },
    complex_path = { "foo[1].bar[2]=value", Separator::Dot, json!({"foo": [null, {"bar": [null, null, "value"]}]}) },
    key_with_spaces = { "\"key with spaces\"=value", Separator::Dot, json!({"key with spaces": "value"}) },
    array_of_objects = { "items[0].name=Item1", Separator::Dot, json!({"items": [{"name": "Item1"}]}) },
    nested_objects = { "obj.level1.level2=value", Separator::Dot, json!({"obj": {"level1": {"level2": "value"}}}) },
    empty_string_value = { "empty=\"\"", Separator::Dot, json!({"empty": ""}) },
    special_chars_in_key = { "\"key!@#$%^&*()\"=value", Separator::Dot, json!({"key!@#$%^&*()": "value"}) },
    unicode_key = { "\"ключ\"=значение", Separator::Dot, json!({"ключ": "значение"}) },
    multiple_arrays = { "a[0][1][2]=value", Separator::Dot, json!({
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
    example_from_readme = { "foo.bar[0].baz=true", Separator::Dot, json!({"foo": {"bar": [{"baz": true}]}}) },
)]
fn test_parse_input_ok(input: &str, separator: Separator, expected: serde_json::Value) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");

    let mut json_obj = serde_json::Value::Null;
    parsed.insert_into(&mut json_obj);

    assert_eq!(json_obj, expected);
}

#[parameterized(
    negative_index = { "arr[-1]=value", Separator::Dot, ParseError::NomError("Parsing Error: VerboseError { errors: [(\"[-1]=value\", Char('='))] }".to_string()) },
    invalid_index = { "arr[invalid]=value", Separator::Dot, ParseError::NomError("Parsing Error: VerboseError { errors: [(\"[invalid]=value\", Char('='))] }".to_string())}, 
    missing_value = { "key=", Separator::Dot, ParseError::NomError("Parsing Error: VerboseError { errors: [(\"\", Nom(IsNot))] }".to_string()) },
    missing_key = { "=value", Separator::Dot, ParseError::NomError("Parsing Error: VerboseError { errors: [(\"=value\", Nom(TakeWhile1)), (\"=value\", Nom(Alt)), (\"=value\", Nom(Alt))] }".to_string()) },
    missing_assignment = { "key", Separator::Dot, ParseError::NomError("Parsing Error: VerboseError { errors: [(\"\", Char('='))] }".to_string()) },
)]
fn test_parse_input_err(input: &str, separator: Separator, expected: ParseError) {
    let result = Jqesque::from_str_with_separator(input, separator);

    match result {
        Ok(_) => {
            let parsed = result.unwrap();
            let mut json_obj = serde_json::Value::Null;
            insert_value(&mut json_obj, parsed.tokens(), parsed.value());
            panic!(
                "Expected an error, but got Ok (tokens: {:?} -> json_obj: {})",
                parsed.tokens(),
                json_obj
            );
        }
        Err(err) => assert_eq!(err, expected),
    }
}

#[parameterized(
    simple_key = { "key=value", json!({"key": "value"}) } ,
    nested_keys = { "parent.child=value", json!({"parent": {"child": "value"}}) },
)]
fn test_using_from_str_with_default_separator(input: &str, expected: serde_json::Value) {
    let parsed = input.parse::<Jqesque>().expect("Failed to parse input");

    let mut json_obj = serde_json::Value::Null;
    parsed.insert_into(&mut json_obj);

    assert_eq!(json_obj, expected);
}

#[parameterized(
    simple_key = { "key=value", json!({"key": "value"}) } ,
    nested_keys = { "parent.child=value", json!({"parent": {"child": "value"}}) },
)]
fn test_as_json(input: &str, expected: serde_json::Value) {
    let json_obj = input
        .parse::<Jqesque>()
        .expect("Failed to parse input")
        .as_json();

    assert_eq!(json_obj, expected);
}
