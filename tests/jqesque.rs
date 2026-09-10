use jqesque::{
    Jqesque, JqesqueError, Operation, ParseOptions, PathToken, Separator, DEFAULT_MAX_ARRAY_INDEX,
    DEFAULT_MAX_PATH_DEPTH,
};
use rstest::rstest;
use serde_json::json;

#[rstest]
#[case("key=value", json!({"key": "value"}))]
#[case("parent.child=value", json!({"parent": {"child": "value"}}))]
fn test_using_from_str_with_default_separator(
    #[case] input: &str,
    #[case] expected: serde_json::Value,
) {
    let parsed = input.parse::<Jqesque>().expect("Failed to parse input");

    let mut json_obj = serde_json::Value::Null;
    parsed.apply_to(&mut json_obj).unwrap();

    assert_eq!(json_obj, expected);
}

#[rstest]
#[case(">key=value", Separator::Dot, json!({"key": "value"}))]
#[case(">parent.child=value", Separator::Dot, json!({"parent": {"child": "value"}}))]
#[case(">array[0]=1", Separator::Dot, json!({"array": [1]}))]
#[case(">array[0][1]=2", Separator::Dot, json!({"array": [[null, 2]]}))]
#[case(">key1/key2=value", Separator::Slash, json!({"key1": {"key2": "value"}}))]
#[case(">\"complex.key\"=123", Separator::Dot, json!({"complex.key": 123}))]
#[case(">flag=true", Separator::Dot, json!({"flag": true}))]
#[case(">nothing=null", Separator::Dot, json!({"nothing": null}))]
#[case(">number=42", Separator::Dot, json!({"number": 42}))]
#[case(">pi=3.14", Separator::Dot, json!({"pi": 314_f64 / 100.0}))]
#[case(">foo[1].bar[2]=value", Separator::Dot, json!({"foo": [null, {"bar": [null, null, "value"]}]}))]
#[case(">\"key with spaces\"=value", Separator::Dot, json!({"key with spaces": "value"}))]
#[case(">items[0].name=Item1", Separator::Dot, json!({"items": [{"name": "Item1"}]}))]
#[case(">obj.level1.level2=value", Separator::Dot, json!({"obj": {"level1": {"level2": "value"}}}))]
#[case(">empty=\"\"", Separator::Dot, json!({"empty": ""}))]
#[case(">\"key!@#$%^&*()\"=value", Separator::Dot, json!({"key!@#$%^&*()": "value"}))]
#[case(">\"ключ\"=значение", Separator::Dot, json!({"ключ": "значение"}))]
#[case(">foo.bar[0].baz=true", Separator::Dot, json!({"foo": {"bar": [{"baz": true}]}}))]
#[case(">arr=[1,2,3]", Separator::Dot, json!({"arr": [1, 2, 3]}))]
#[case(">arr=[\"a\",\"b\",\"c\"]", Separator::Dot, json!({"arr": ["a", "b", "c"]}))]
#[case(r#">arr=["a","b","c"]"#, Separator::Dot, json!({"arr": ["a", "b", "c"]}))]
fn test_parse_input_insert_ok(
    #[case] input: &str,
    #[case] separator: Separator,
    #[case] expected: serde_json::Value,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");

    let mut json_obj = serde_json::Value::Null;
    parsed.apply_to(&mut json_obj).unwrap();

    assert_eq!(json_obj, expected);
}

#[rstest]
#[case("+parent.child.key=value", Separator::Dot, json!({"parent": {"child": {"key": "value"}}, "array": [1, 2, 3]}))]
#[case("+array/1=42", Separator::Slash, json!({"parent": {"child": {}}, "array": [1, 42, 2, 3]}))]
#[case("+array/-=99", Separator::Slash, json!({"parent": {"child": {}}, "array": [1, 2, 3, 99]}))]
fn test_add_operation_success(
    #[case] input: &str,
    #[case] separator: Separator,
    #[case] expected: serde_json::Value,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), &Operation::Add);

    let mut json_obj = json!({
        "parent": {
            "child": {}
        },
        "array": [1, 2, 3]
    });

    parsed.apply_to(&mut json_obj).unwrap();

    assert_eq!(json_obj, expected);
}

#[rstest]
#[case("+nonexistent.key=value", Separator::Dot)]
#[case("+nonexistent_array/0=value", Separator::Slash)]
#[case("+array/10=value", Separator::Slash)]
fn test_add_operation_failure(#[case] input: &str, #[case] separator: Separator) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), &Operation::Add);

    let mut json_obj = json!({
        "parent": {
            "child": {}
        },
        "array": [1, 2, 3]
    });

    let result = parsed.apply_to(&mut json_obj);
    assert!(result.is_err(), "Expected error but operation succeeded");
}

#[rstest]
#[case("=parent.child.key=new_value", Separator::Dot, json!({"parent": {"child": {"key": "new_value"}}, "array": [1, 2, 3], "root": "root_value"}))]
#[case("=array/1=42", Separator::Slash, json!({"parent": {"child": {"key": "old_value"}}, "array": [1, 42, 3], "root": "root_value"}))]
#[case("=root=new_root_value", Separator::Dot, json!({"parent": {"child": {"key": "old_value"}}, "array": [1, 2, 3], "root": "new_root_value"}))]
#[case("=parent={\"new\": \"object\"}", Separator::Dot, json!({"parent": {"new": "object"}, "array": [1, 2, 3], "root": "root_value"}))]
#[case("=parent.child.key=null", Separator::Dot, json!({"parent": {"child": {"key": null}}, "array": [1, 2, 3], "root": "root_value"}))]
fn test_replace_operation_success(
    #[case] input: &str,
    #[case] separator: Separator,
    #[case] expected: serde_json::Value,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), &Operation::Replace);

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

#[rstest]
#[case("=parent.child.nonexistent_key=new_value", Separator::Dot)]
#[case("=array/10=42", Separator::Slash)]
#[case("=nonexistent_root_key=value", Separator::Dot)]
#[case("=array/1/key=value", Separator::Slash)]
fn test_replace_operation_failure(#[case] input: &str, #[case] separator: Separator) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), &Operation::Replace);

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

#[rstest]
#[case("-existing_key", Separator::Dot, json!({"array": [1, 2, 3], "nested": {"key": "value", "array": [1, 2, 3]}}))]
#[case("-nested.key", Separator::Dot, json!({"existing_key": "value", "array": [1, 2, 3], "nested": {"array": [1, 2, 3]}}))]
#[case("-array[1]", Separator::Dot, json!({"existing_key": "value", "array": [1, 3], "nested": {"key": "value", "array": [1, 2, 3]}}))]
#[case("-array", Separator::Dot, json!({"existing_key": "value", "nested": {"key": "value", "array": [1, 2, 3]}}))]
#[case("-nested.array[0]", Separator::Dot, json!({"existing_key": "value", "array": [1, 2, 3], "nested": {"key": "value", "array": [2, 3]}}))]
fn test_remove_operation_success(
    #[case] input: &str,
    #[case] separator: Separator,
    #[case] expected: serde_json::Value,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), &Operation::Remove);

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

#[rstest]
#[case("-nonexistent_key", Separator::Dot)]
#[case("-nested.nonexistent_key", Separator::Dot)]
#[case("-array[10]", Separator::Dot)]
#[case("-nonexistent_array[0]", Separator::Dot)]
fn test_remove_operation_failure(#[case] input: &str, #[case] separator: Separator) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), &Operation::Remove);

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

#[rstest]
#[case("existing_key=new_value", Separator::Dot, json!({"existing_key": "new_value", "array": [1, 2, 3]}), Operation::Replace)]
#[case("new_key=new_value", Separator::Dot, json!({"existing_key": "old_value", "array": [1, 2, 3], "new_key": "new_value"}), Operation::Add)]
#[case("parent.new_child=new_value", Separator::Dot, json!({"existing_key": "old_value", "array": [1, 2, 3], "parent": {"new_child": "new_value"}}), Operation::Insert)]
#[case("array[1]=42", Separator::Dot, json!({"existing_key": "old_value", "array": [1, 42, 3]}), Operation::Replace)]
#[case("array[3]=4", Separator::Dot, json!({"existing_key": "old_value", "array": [1, 2, 3, 4]}), Operation::Add)]
#[case("new_array[0]=1", Separator::Dot, json!({"existing_key": "old_value", "array": [1, 2, 3], "new_array": [1]}), Operation::Insert)]
fn test_auto_operation(
    #[case] input: &str,
    #[case] separator: Separator,
    #[case] expected: serde_json::Value,
    #[case] expected_operation: Operation,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), &Operation::Auto);

    let mut json_obj = json!({
        "existing_key": "old_value",
        "array": [1, 2, 3]
    });

    let operation = parsed.apply_to(&mut json_obj).unwrap();
    assert_eq!(operation, expected_operation);

    assert_eq!(json_obj, expected);
}

#[rstest]
#[case("-nonexistent", Separator::Dot)]
#[case("=nonexistent=value", Separator::Dot)]
#[case("+array[10]=value", Separator::Dot)]
fn test_patch_errors(#[case] input: &str, #[case] separator: Separator) {
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

#[rstest]
#[case("?key=value", Separator::Dot, json!({ "key": "value" }))]
#[case("?parent.child=value", Separator::Dot, json!({ "parent": { "child": "value" } }))]
#[case("?array[0]=1", Separator::Dot, json!({ "array": [1, 2, 3] }))]
#[case("?array[0][1]=2", Separator::Dot, json!({ "array": [[null, 2]] }))]
fn test_test_operation_success(
    #[case] input: &str,
    #[case] separator: Separator,
    #[case] initial_json: serde_json::Value,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), &Operation::Test);

    let mut json_obj = initial_json;
    assert!(parsed.apply_to(&mut json_obj).is_ok());
}

#[rstest]
#[case("?key=expected_value", Separator::Dot, json!({ "key": "actual_value" }))]
#[case("?nonexistent=value", Separator::Dot, json!({ "key": "value" }))]
fn test_test_failed_errors(
    #[case] input: &str,
    #[case] separator: Separator,
    #[case] initial_json: serde_json::Value,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).unwrap();
    let mut json_obj = initial_json;
    let result = parsed.apply_to(&mut json_obj);
    assert!(
        result.is_err(),
        "Expected TestFailedError but operation succeeded"
    );
    match result {
        Err(JqesqueError::TestFailedError { .. }) => (),
        Err(JqesqueError::InvalidPathError(_)) => (),
        Err(e) => panic!("Expected TestFailedError, got {:?}", e),
        _ => panic!("Expected error but operation succeeded"),
    }
}

#[rstest]
#[case("+key..subkey=value", Separator::Dot)]
#[case("+array[-1]=value", Separator::Dot)]
#[case("+key\\subkey=value", Separator::Dot)]
fn test_invalid_path_errors(#[case] input: &str, #[case] separator: Separator) {
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

#[rstest]
#[case(">key=value", json!({"key": "value"}))]
#[case(">parent.child=value", json!({"parent": {"child": "value"}}))]
fn test_as_json(#[case] input: &str, #[case] expected: serde_json::Value) {
    let json_obj = input
        .parse::<Jqesque>()
        .expect("Failed to parse input")
        .as_json();

    assert_eq!(json_obj, expected);
}

#[rstest]
#[case("key=value")]
#[case("key={\"missing\":}")]
#[case("key=[1,]")]
#[case("key=true trailing")]
fn test_strict_json_values_reports_json_errors(#[case] input: &str) {
    let options = ParseOptions::new(Separator::Dot).strict_json_values(true);
    let parsed = Jqesque::from_str_with_options(input, options);
    match parsed {
        Err(JqesqueError::InvalidJsonValueError(message)) => {
            assert!(message.contains("line 1 column"), "{message}");
        }
        result => panic!("Expected a JSON value error, got {result:?}"),
    }
}

#[rstest]
#[case("key..child=1")]
#[case("key=")]
#[case("\"Invalid JSON value in strict mode:\"..child=1")]
fn test_strict_json_values_preserves_syntax_errors(#[case] input: &str) {
    let options = ParseOptions::new(Separator::Dot).strict_json_values(true);
    let parsed = Jqesque::from_str_with_options(input, options);
    assert!(matches!(parsed, Err(JqesqueError::NomError(_))));
}

#[test]
fn test_strict_json_values_accepts_valid_json() {
    let options = ParseOptions::new(Separator::Dot).strict_json_values(true);
    let parsed = Jqesque::from_str_with_options("key=\"value\"", options).unwrap();

    let mut json_obj = serde_json::Value::Null;
    parsed.apply_to(&mut json_obj).unwrap();

    assert_eq!(json_obj, json!({"key": "value"}));
}

#[rstest]
#[case("a.b.c=1", Separator::Dot, 2)]
#[case("a[0][0]=1", Separator::Dot, 2)]
#[case("[0][0][0]=1", Separator::Dot, 2)]
#[case("a[0].b=1", Separator::Dot, 2)]
#[case("a/[0]/b=1", Separator::Slash, 2)]
#[case("a:[0]:b=1", Separator::Custom(':'), 2)]
#[case("\"a.b\".\"c.d\"[0]=1", Separator::Dot, 2)]
#[case("-a.b.c", Separator::Dot, 2)]
#[case("a=1", Separator::Dot, 0)]
#[case("[0]=1", Separator::Dot, 0)]
fn test_max_path_depth_limit(
    #[case] input: &str,
    #[case] separator: Separator,
    #[case] depth: usize,
) {
    let options = ParseOptions::new(separator).max_path_depth(depth);
    let parsed = Jqesque::from_str_with_options(input, options);

    assert!(matches!(
        parsed,
        Err(JqesqueError::LimitExceededError {
            kind: "path depth",
            limit,
            found,
        }) if limit == depth && found == depth + 1
    ));
}

#[rstest]
#[case(".a")]
#[case("[0]")]
#[case(".\"a.b\"")]
fn test_path_depth_stops_at_first_excess_token(#[case] segment: &str) {
    // The malformed suffix must never be reached, even in permissive value mode.
    let input = format!("a{}[invalid]=not json", segment.repeat(100_000));
    let options = ParseOptions::new(Separator::Dot).max_path_depth(2);
    assert!(matches!(
        Jqesque::from_str_with_options(&input, options),
        Err(JqesqueError::LimitExceededError {
            kind: "path depth",
            limit: 2,
            found: 3,
        })
    ));
}

#[rstest]
#[case(false)]
#[case(true)]
fn test_path_depth_checked_before_value_decoding(#[case] strict: bool) {
    let input = format!("a.b.c={}", "not json".repeat(100_000));
    let options = ParseOptions::new(Separator::Dot)
        .max_path_depth(2)
        .strict_json_values(strict);
    assert!(matches!(
        Jqesque::from_str_with_options(&input, options),
        Err(JqesqueError::LimitExceededError {
            kind: "path depth",
            limit: 2,
            found: 3,
        })
    ));
}

#[rstest]
#[case(DEFAULT_MAX_PATH_DEPTH)]
#[case(usize::MAX)]
fn test_effective_path_depth_ceiling_stops_tokenization(#[case] requested: usize) {
    let input = format!("a{}=not json", ".a".repeat(100_000));
    let options = ParseOptions::new(Separator::Dot)
        .max_path_depth(requested)
        .strict_json_values(true);
    assert!(matches!(
        Jqesque::from_str_with_options(&input, options),
        Err(JqesqueError::LimitExceededError {
            kind: "path depth",
            limit: DEFAULT_MAX_PATH_DEPTH,
            found,
        }) if found == DEFAULT_MAX_PATH_DEPTH + 1
    ));
}

#[rstest]
#[case("a.b.=1")]
#[case("a..b=1")]
#[case("a[bad]=1")]
#[case("a[0=1")]
#[case("a[999999999999999999999999999999999999]=1")]
#[case("a.\"unterminated=1")]
fn test_malformed_path_within_depth_limit(#[case] input: &str) {
    let options = ParseOptions::new(Separator::Dot).max_path_depth(2);
    assert!(matches!(
        Jqesque::from_str_with_options(input, options),
        Err(JqesqueError::NomError(_))
    ));
}

#[rstest]
#[case("a[0][1]=1", Separator::Dot, 3)]
#[case("[0][1]=1", Separator::Dot, 2)]
#[case("\"a.b\"[0]=1", Separator::Dot, 2)]
#[case("a/[0]=1", Separator::Slash, 2)]
#[case("a:[0]=1", Separator::Custom(':'), 2)]
#[case("a={\"b\":1}", Separator::Custom('='), 1)]
#[case("a=[]", Separator::Custom('='), 1)]
#[case("a=\"unterminated", Separator::Custom('='), 1)]
#[case("-a[0]", Separator::Dot, 2)]
fn test_path_tokenization_accepts_exact_depth(
    #[case] input: &str,
    #[case] separator: Separator,
    #[case] depth: usize,
) {
    let options = ParseOptions::new(separator).max_path_depth(depth);
    let parsed = Jqesque::from_str_with_options(input, options).unwrap();
    assert_eq!(parsed.tokens().len(), depth);
}

#[test]
fn test_default_path_depth_accepts_exact_ceiling() {
    let input = format!("a{}=1", ".a".repeat(DEFAULT_MAX_PATH_DEPTH - 1));
    let parsed: Jqesque = input.parse().unwrap();
    assert_eq!(parsed.tokens().len(), DEFAULT_MAX_PATH_DEPTH);
}

#[test]
fn test_max_array_index_limit() {
    let options = ParseOptions::new(Separator::Dot).max_array_index(10);
    let parsed = Jqesque::from_str_with_options("a[11]=1", options);

    assert!(matches!(
        parsed,
        Err(JqesqueError::LimitExceededError {
            kind: "array index",
            ..
        })
    ));
}

#[rstest]
#[case(">a.b=1", 2, 0, json!({"a": {"b": 1}}))]
#[case(">a[2]=1", 2, 2, json!({"a": [null, null, 1]}))]
#[case(">[0]=1", 1, 0, json!([1]))]
fn test_parse_limits_accept_boundary_values(
    #[case] input: &str,
    #[case] depth: usize,
    #[case] index: usize,
    #[case] expected: serde_json::Value,
) {
    let options = ParseOptions::new(Separator::Dot)
        .max_path_depth(depth)
        .max_array_index(index);
    let parsed = Jqesque::from_str_with_options(input, options).unwrap();
    assert_eq!(parsed.as_json(), expected);
}

#[test]
fn test_new_rejects_out_of_bounds_constructed_index() {
    let parsed = Jqesque::new(
        vec![
            PathToken::Key("a".to_string()),
            PathToken::Index(DEFAULT_MAX_ARRAY_INDEX + 1),
        ],
        Some(json!(1)),
        Operation::Insert,
    );

    assert!(matches!(
        parsed,
        Err(JqesqueError::LimitExceededError {
            kind: "array index",
            ..
        })
    ));
}

#[rstest]
#[case(vec![PathToken::Index(DEFAULT_MAX_ARRAY_INDEX + 1)])]
#[case(vec![PathToken::Index(usize::MAX)])]
#[case(vec![PathToken::Key("a".into()); jqesque::DEFAULT_MAX_PATH_DEPTH + 1])]
fn test_deserialization_rejects_paths_exceeding_safety_limits(#[case] tokens: Vec<PathToken>) {
    let serialized = json!({
        "tokens": tokens,
        "value": 1,
        "operation": "Insert",
    });
    let error = serde_json::from_value::<Jqesque>(serialized).unwrap_err();
    assert!(error.to_string().contains("Limit exceeded"), "{error}");
}

#[rstest]
#[case(">a[2].b=1")]
#[case("~a={\"b\":true}")]
#[case("-a[0]")]
fn test_serialization_round_trip(#[case] input: &str) {
    let parsed: Jqesque = input.parse().unwrap();
    let serialized = serde_json::to_value(&parsed).unwrap();
    let restored: Jqesque = serde_json::from_value(serialized).unwrap();
    assert_eq!(restored, parsed);
    assert_eq!(restored.as_json(), parsed.as_json());
}

#[rstest]
#[case(format!(">a[{}]=1", DEFAULT_MAX_ARRAY_INDEX + 1))]
#[case(format!(">{}=1", vec!["a"; jqesque::DEFAULT_MAX_PATH_DEPTH + 1].join(".")))]
fn test_parse_options_cannot_bypass_safety_limits(#[case] input: String) {
    let options = ParseOptions::new(Separator::Dot)
        .max_path_depth(usize::MAX)
        .max_array_index(usize::MAX);
    assert!(matches!(
        Jqesque::from_str_with_options(&input, options),
        Err(JqesqueError::LimitExceededError { .. })
    ));
}
