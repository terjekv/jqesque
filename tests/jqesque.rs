use jqesque::{
    ApplyOptions, Batch, Jqesque, JqesqueError, LimitKind, Operation, ParseOptions, Path,
    PathErrorKind, PathToken, Separator, SyntaxKind, DEFAULT_MAX_ARRAY_INDEX,
    DEFAULT_MAX_PATH_DEPTH,
};
use rstest::rstest;
use serde_json::{json, Value};

#[rstest]
#[case("key=value", json!({"key": "value"}))]
#[case("parent.child=value", json!({"parent": {"child": "value"}}))]
fn test_using_from_str_with_default_separator(#[case] input: &str, #[case] expected: Value) {
    let parsed = input.parse::<Jqesque>().expect("Failed to parse input");

    let mut json_obj = Value::Null;
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
    #[case] expected: Value,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");

    let mut json_obj = Value::Null;
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
    #[case] expected: Value,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), Operation::Add);

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
    assert_eq!(parsed.operation(), Operation::Add);

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
    #[case] expected: Value,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), Operation::Replace);

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
    assert_eq!(parsed.operation(), Operation::Replace);

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
    #[case] expected: Value,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), Operation::Remove);

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
    assert_eq!(parsed.operation(), Operation::Remove);

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
    #[case] expected: Value,
    #[case] expected_operation: Operation,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), Operation::Auto);

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
        "Expected PathError but operation succeeded"
    );
    match result {
        Err(JqesqueError::PathError { .. }) => (),
        Err(e) => panic!("Expected PathError, got {:?}", e),
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
    #[case] initial_json: Value,
) {
    let parsed = Jqesque::from_str_with_separator(input, separator).expect("Failed to parse input");
    assert_eq!(parsed.operation(), Operation::Test);

    let mut json_obj = initial_json;
    assert!(parsed.apply_to(&mut json_obj).is_ok());
}

#[rstest]
#[case("?key=expected_value", Separator::Dot, json!({ "key": "actual_value" }))]
#[case("?nonexistent=value", Separator::Dot, json!({ "key": "value" }))]
fn test_test_failed_errors(
    #[case] input: &str,
    #[case] separator: Separator,
    #[case] initial_json: Value,
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
        Err(JqesqueError::PathError { .. }) => (),
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
        Err(JqesqueError::SyntaxError { .. }) => (),
        Err(e) => panic!("Expected SyntaxError, got {:?}", e),
        _ => panic!("Expected error but parsing succeeded"),
    }
}

#[rstest]
#[case(">key=value", json!({"key": "value"}))]
#[case(">parent.child=value", json!({"parent": {"child": "value"}}))]
fn test_document_materialization(#[case] input: &str, #[case] expected: Value) {
    let json_obj = input
        .parse::<Jqesque>()
        .expect("Failed to parse input")
        .to_document()
        .unwrap();

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
        Err(JqesqueError::InvalidJsonValueError { message, .. }) => {
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
    assert!(matches!(parsed, Err(JqesqueError::SyntaxError { .. })));
}

#[test]
fn test_strict_json_values_accepts_valid_json() {
    let options = ParseOptions::new(Separator::Dot).strict_json_values(true);
    let parsed = Jqesque::from_str_with_options("key=\"value\"", options).unwrap();

    let mut json_obj = Value::Null;
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
            kind: LimitKind::PathDepth,
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
            kind: LimitKind::PathDepth,
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
            kind: LimitKind::PathDepth,
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
            kind: LimitKind::PathDepth,
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
        Err(JqesqueError::SyntaxError { .. })
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
            kind: LimitKind::ArrayIndex,
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
    #[case] expected: Value,
) {
    let options = ParseOptions::new(Separator::Dot)
        .max_path_depth(depth)
        .max_array_index(index);
    let parsed = Jqesque::from_str_with_options(input, options).unwrap();
    assert_eq!(parsed.to_document().unwrap(), expected);
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
            kind: LimitKind::ArrayIndex,
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
    assert!(error.to_string().contains("limit exceeded"), "{error}");
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

#[rstest]
#[case(r#"+"a/b"=1"#, "a/b")]
#[case(r#"+"a~b"=1"#, "a~b")]
#[case(r#"+"~1/~0"=1"#, "~1/~0")]
fn patch_keys_are_escaped_once(#[case] input: &str, #[case] key: &str) {
    let assignment: Jqesque = input.parse().unwrap();
    let mut document = json!({});
    assignment.apply_to(&mut document).unwrap();
    assert_eq!(document, json!({key: 1}));
}

#[rstest]
#[case("+key=null")]
#[case("=key=null")]
#[case("?key=null")]
#[case("key=null")]
#[case(">key=null")]
#[case("~key=null")]
#[case("merge-patch key=null")]
fn explicit_null_survives_serialization(#[case] input: &str) {
    let assignment: Jqesque = input.parse().unwrap();
    let restored: Jqesque =
        serde_json::from_str(&serde_json::to_string(&assignment).unwrap()).unwrap();
    assert_eq!(restored, assignment);
    let mut document = json!({"key": null});
    restored.apply_to(&mut document).unwrap();
    assert_eq!(document, json!({"key": null}));
}

#[rstest]
#[case(r#">"a\nb"=1"#, "a\nb")]
#[case(r#">"a\tb"=1"#, "a\tb")]
#[case(r#">"a\rb"=1"#, "a\rb")]
#[case(r#">"\u0061"=1"#, "a")]
#[case(r#">"\uD83D\uDE00"=1"#, "😀")]
#[case(r#">""=1"#, "")]
#[case(r#">"\\\"/\b\f"=1"#, "\\\"/\u{8}\u{c}")]
fn quoted_keys_follow_json_string_rules(#[case] input: &str, #[case] key: &str) {
    assert_eq!(
        input.parse::<Jqesque>().unwrap().tokens(),
        &[PathToken::Key(key.into())]
    );
}

#[rstest]
#[case(r#">"\uD800"=1"#)]
#[case(r#">"\x"=1"#)]
#[case(">\"line\nfeed\"=1")]
fn invalid_quoted_keys_are_rejected(#[case] input: &str) {
    assert!(matches!(
        input.parse::<Jqesque>(),
        Err(JqesqueError::SyntaxError {
            kind: SyntaxKind::InvalidQuotedKey,
            ..
        })
    ));
}

#[rstest]
#[case("~a[1]=99", json!({"a":[10,20,30]}), json!({"a":[10,99,30]}))]
#[case("~a[2].x=1", json!({"a":[10,20,{"y":2}]}), json!({"a":[10,20,{"x":1,"y":2}]}))]
#[case("~a[3]=99", json!({"a":[10,20]}), json!({"a":[10,20,null,99]}))]
#[case("~a=[null]", json!({"a":[10,20]}), json!({"a":[null,20]}))]
#[case("~a={\"x\":null}", json!({"a":{"x":1,"y":2}}), json!({"a":{"x":null,"y":2}}))]
#[case("~a={\"x\":null}", json!({"a":1}), json!({"a":{"x":null}}))]
fn deep_merge_preserves_unaddressed_content(
    #[case] input: &str,
    #[case] mut document: Value,
    #[case] expected: Value,
) {
    input
        .parse::<Jqesque>()
        .unwrap()
        .apply_to(&mut document)
        .unwrap();
    assert_eq!(document, expected);
}

// RFC 7396 Appendix A conformance cases, applied at the document root.
#[rstest]
#[case(json!({"a":"b"}), json!({"a":"c"}), json!({"a":"c"}))]
#[case(json!({"a":"b"}), json!({"b":"c"}), json!({"a":"b","b":"c"}))]
#[case(json!({"a":"b"}), json!({"a":null}), json!({}))]
#[case(json!({"a":"b","b":"c"}), json!({"a":null}), json!({"b":"c"}))]
#[case(json!({"a":["b"]}), json!({"a":"c"}), json!({"a":"c"}))]
#[case(json!({"a":"c"}), json!({"a":["b"]}), json!({"a":["b"]}))]
#[case(json!({"a":{"b":"c"}}), json!({"a":{"b":"d","c":null}}), json!({"a":{"b":"d"}}))]
#[case(json!({"a":[{"b":"c"}]}), json!({"a":[1]}), json!({"a":[1]}))]
#[case(json!(["a","b"]), json!(["c","d"]), json!(["c","d"]))]
#[case(json!({"a":"b"}), json!(["c"]), json!(["c"]))]
#[case(json!({"a":"foo"}), json!(null), json!(null))]
#[case(json!({"a":"foo"}), json!("bar"), json!("bar"))]
#[case(json!({"e":null}), json!({"a":1}), json!({"e":null,"a":1}))]
#[case(json!([1,2]), json!({"a":"b","c":null}), json!({"a":"b"}))]
#[case(json!({}), json!({"a":{"bb":{"ccc":null}}}), json!({"a":{"bb":{}}}))]
fn rfc7396_root_merge_patch(
    #[case] mut document: Value,
    #[case] patch: Value,
    #[case] expected: Value,
) {
    let assignment = Jqesque::from_path(Path::root(), Some(patch), Operation::MergePatch).unwrap();
    assignment.apply_to(&mut document).unwrap();
    assert_eq!(document, expected);
}

#[rstest]
#[case("merge-patch a={\"x\":null}", json!({"a":{"x":1,"y":2}}), json!({"a":{"y":2}}))]
#[case("merge-patch a=[9]", json!({"a":[1,2]}), json!({"a":[9]}))]
#[case("merge-patch a=null", json!({"a":1}), json!({"a":null}))]
#[case("merge-patch a[1]={\"x\":null}", json!({"a":[1,{"x":2,"y":3}]}), json!({"a":[1,{"y":3}]}))]
fn merge_patch_selected_value(
    #[case] input: &str,
    #[case] mut document: Value,
    #[case] expected: Value,
) {
    input
        .parse::<Jqesque>()
        .unwrap()
        .apply_to(&mut document)
        .unwrap();
    assert_eq!(document, expected);
}

#[rstest]
#[case("1", "1.0", true)]
#[case("0", "-0.0", true)]
#[case("100", "1e2", true)]
#[case("9007199254740993", "9007199254740992.0", false)]
#[case("18446744073709551615", "18446744073709551616.0", false)]
#[case("-9223372036854775808", "-9223372036854775808.0", true)]
#[case("{\"a\":[1,0]}", "{\"a\":[1.0,-0.0]}", true)]
#[case("[1]", "[2]", false)]
#[case("{\"a\":1}", "{\"b\":1}", false)]
fn json_patch_numeric_equality(#[case] actual: &str, #[case] expected: &str, #[case] equal: bool) {
    let mut document = serde_json::from_str(actual).unwrap();
    let assignment = Jqesque::from_path(
        Path::root(),
        Some(serde_json::from_str(expected).unwrap()),
        Operation::Test,
    )
    .unwrap();
    assert_eq!(assignment.apply_to(&mut document).is_ok(), equal);
}

#[rstest]
#[case("insert a=1", ">a=1")]
#[case("merge a=1", "~a=1")]
#[case("add a=1", "+a=1")]
#[case("replace a=1", "=a=1")]
#[case("remove a", "-a")]
#[case("test a=1", "?a=1")]
#[case("auto a=1", "a=1")]
#[case("insert\t  a=1", ">a=1")]
fn named_operations_match_shorthand(#[case] named: &str, #[case] short: &str) {
    assert_eq!(
        named.parse::<Jqesque>().unwrap(),
        short.parse::<Jqesque>().unwrap()
    );
}

#[rstest]
#[case("insert=1", "insert")]
#[case("merge.child=1", "merge")]
#[case("remove[0]=1", "remove")]
fn operation_names_remain_valid_keys(#[case] input: &str, #[case] key: &str) {
    assert_eq!(
        input.parse::<Jqesque>().unwrap().tokens()[0],
        PathToken::Key(key.into())
    );
}

#[rstest]
#[case("insert .=1", Operation::Insert)]
#[case("merge-patch .={}", Operation::MergePatch)]
#[case(".=null", Operation::Auto)]
fn root_path_syntax(#[case] input: &str, #[case] operation: Operation) {
    let assignment: Jqesque = input.parse().unwrap();
    assert!(assignment.path().is_root());
    assert_eq!(assignment.operation(), operation);
}

#[rstest]
#[case(Operation::Add)]
#[case(Operation::Replace)]
#[case(Operation::Test)]
#[case(Operation::Insert)]
#[case(Operation::Merge)]
#[case(Operation::MergePatch)]
#[case(Operation::Auto)]
fn constructors_reject_missing_values(#[case] operation: Operation) {
    assert_eq!(
        Jqesque::from_path(Path::root(), None, operation),
        Err(JqesqueError::MissingValueError(operation))
    );
}

#[test]
fn constructor_rejects_remove_value() {
    assert_eq!(
        Jqesque::from_path(Path::root(), Some(json!(1)), Operation::Remove),
        Err(JqesqueError::UnexpectedValueError(Operation::Remove))
    );
}

#[test]
fn legacy_remove_serialization_is_accepted() {
    let assignment: Jqesque =
        serde_json::from_value(json!({"tokens":[{"Key":"a"}],"value":null,"operation":"Remove"}))
            .unwrap();
    assert_eq!(assignment, "-a".parse().unwrap());
    assert!(serde_json::to_value(assignment)
        .unwrap()
        .get("value")
        .is_none());
}

#[rstest]
#[case(json!({"tokens":[],"operation":"Add"}))]
#[case(json!({"tokens":[],"value":1,"operation":"Remove"}))]
fn deserialization_enforces_operation_values(#[case] serialized: Value) {
    assert!(serde_json::from_value::<Jqesque>(serialized).is_err());
}

#[rstest]
#[case("a=1")]
#[case("~a=1")]
#[case("merge-patch a=1")]
#[case(">a=1")]
fn patch_export_rejects_non_patch_operations(#[case] input: &str) {
    let assignment: Jqesque = input.parse().unwrap();
    assert_eq!(
        assignment.to_json_patch(),
        Err(JqesqueError::UnsupportedConversion(assignment.operation()))
    );
}

#[test]
fn patch_export_has_standard_shape() {
    assert_eq!(
        r#"+"a/b"=null"#.parse::<Jqesque>().unwrap().to_json_patch().unwrap(),
        json!([{"op":"add","path":"/a~1b","value":null}])
    );
}

#[rstest]
#[case("bogus a=1", SyntaxKind::UnknownOperation, 0, 1)]
#[case("a[bad]=1", SyntaxKind::InvalidArrayIndex, 2, 3)]
#[case("é[bad]=1", SyntaxKind::InvalidArrayIndex, 3, 3)]
#[case("a..b=1", SyntaxKind::ExpectedPath, 2, 3)]
#[case("a=", SyntaxKind::ExpectedValue, 2, 3)]
fn syntax_errors_include_source_locations(
    #[case] input: &str,
    #[case] kind: SyntaxKind,
    #[case] offset: usize,
    #[case] column: usize,
) {
    let Err(JqesqueError::SyntaxError {
        kind: actual,
        location,
    }) = input.parse::<Jqesque>()
    else {
        panic!("expected syntax error");
    };
    assert_eq!(actual, kind);
    assert_eq!(location.offset(), offset);
    assert_eq!(location.column(), column);
}

#[rstest]
#[case("=missing=1", PathErrorKind::MissingPath)]
#[case("+a.foo=1", PathErrorKind::InvalidIndex)]
#[case("+a[5]=1", PathErrorKind::IndexOutOfBounds)]
#[case("+x.y=1", PathErrorKind::TypeConflict)]
fn runtime_errors_are_structured_and_do_not_mutate(
    #[case] input: &str,
    #[case] kind: PathErrorKind,
) {
    let mut document = json!({"a":[1],"x":1});
    let before = document.clone();
    let result = input.parse::<Jqesque>().unwrap().apply_to(&mut document);
    assert!(matches!(result, Err(JqesqueError::PathError { kind: actual, .. }) if actual == kind));
    assert_eq!(document, before);
}

#[rstest]
#[case(3, true)]
#[case(2, false)]
fn input_byte_limit_boundary(#[case] limit: usize, #[case] accepted: bool) {
    let result =
        Jqesque::from_str_with_options("a=1", ParseOptions::default().max_input_bytes(limit));
    assert_eq!(result.is_ok(), accepted);
    if !accepted {
        assert!(matches!(
            result,
            Err(JqesqueError::LimitExceededError {
                kind: LimitKind::InputBytes,
                ..
            })
        ));
    }
}

#[test]
fn array_index_rejected_before_json_decoding() {
    assert!(matches!(
        Jqesque::from_str_with_options(
            "a[11]=not json",
            ParseOptions::default()
                .strict_json_values(true)
                .max_array_index(10)
        ),
        Err(JqesqueError::LimitExceededError {
            kind: LimitKind::ArrayIndex,
            ..
        })
    ));
}

#[test]
fn cumulative_sparse_path_slots_are_bounded() {
    let path = vec![
        PathToken::Index(DEFAULT_MAX_ARRAY_INDEX),
        PathToken::Index(0),
    ];
    assert!(matches!(
        Path::new(path),
        Err(JqesqueError::LimitExceededError {
            kind: LimitKind::ArraySlots,
            ..
        })
    ));
}

#[rstest]
#[case(4, true)]
#[case(3, false)]
fn application_array_budget_precedes_mutation(#[case] limit: usize, #[case] accepted: bool) {
    let assignment: Jqesque = ">a[2][1]=[1]".parse().unwrap();
    let mut document = json!({"a":[0,1]});
    let before = document.clone();
    let result =
        assignment.apply_to_with_options(&mut document, ApplyOptions::new().max_array_slots(limit));
    assert_eq!(result.is_ok(), accepted);
    if !accepted {
        assert_eq!(document, before);
    }
}

#[test]
fn budget_failure_does_not_trigger_auto_insert_fallback() {
    let mut document = json!({"a":[1]});
    let result = "a[1]=2"
        .parse::<Jqesque>()
        .unwrap()
        .apply_to_with_options(&mut document, ApplyOptions::new().max_array_slots(0));
    assert!(matches!(
        result,
        Err(JqesqueError::LimitExceededError { .. })
    ));
    assert_eq!(document, json!({"a":[1]}));
}

#[rstest]
#[case(jqesque::DEFAULT_MAX_VALUE_DEPTH, true)]
#[case(jqesque::DEFAULT_MAX_VALUE_DEPTH + 1, false)]
fn constructed_value_depth_is_bounded(#[case] depth: usize, #[case] accepted: bool) {
    let value = (0..depth).fold(json!(0), |value, _| json!([value]));
    let result = Jqesque::from_path(Path::root(), Some(value), Operation::Insert);
    assert_eq!(result.is_ok(), accepted);
}

#[test]
fn atomic_batch_rolls_back_earlier_successes() {
    let batch = Batch::parse(["a=2", "?a=3"], ParseOptions::default()).unwrap();
    let mut document = json!({"a":1});
    assert_eq!(
        batch
            .apply_atomically(&mut document, ApplyOptions::default())
            .unwrap_err()
            .index,
        1
    );
    assert_eq!(document, json!({"a":1}));
}

#[test]
fn ordered_batch_keeps_earlier_successes() {
    let batch = Batch::parse(["a=2", "?a=3"], ParseOptions::default()).unwrap();
    let mut document = json!({"a":1});
    assert_eq!(
        batch
            .apply_to(&mut document, ApplyOptions::default())
            .unwrap_err()
            .index,
        1
    );
    assert_eq!(document, json!({"a":2}));
}

#[test]
fn successful_batch_reports_concrete_operations() {
    let batch = Batch::parse(["a=2", "b=3", "c.d=4", "?a=2.0"], ParseOptions::default()).unwrap();
    let mut document = json!({"a":1});
    assert_eq!(
        batch
            .apply_atomically(&mut document, ApplyOptions::default())
            .unwrap(),
        vec![
            Operation::Replace,
            Operation::Add,
            Operation::Insert,
            Operation::Test
        ]
    );
    assert_eq!(document, json!({"a":2,"b":3,"c":{"d":4}}));
}

#[test]
fn batch_parsing_reports_assignment_index() {
    assert_eq!(
        Batch::parse(["a=1", "bad"], ParseOptions::default())
            .unwrap_err()
            .index,
        1
    );
}

#[test]
fn batch_budget_is_cumulative() {
    let batch = Batch::parse([">a[1]=1", ">b[1]=2"], ParseOptions::default()).unwrap();
    let mut document = json!({});
    let error = batch
        .apply_atomically(&mut document, ApplyOptions::new().max_array_slots(3))
        .unwrap_err();
    assert_eq!(error.index, 1);
    assert!(matches!(
        error.source,
        JqesqueError::LimitExceededError {
            kind: LimitKind::ArraySlots,
            found: 4,
            ..
        }
    ));
    assert_eq!(document, json!({}));
}

#[rstest]
#[case(0, true)]
#[case(jqesque::DEFAULT_MAX_BATCH_LENGTH, true)]
#[case(jqesque::DEFAULT_MAX_BATCH_LENGTH + 1, false)]
fn batch_length_is_bounded(#[case] length: usize, #[case] accepted: bool) {
    assert_eq!(
        Batch::parse(std::iter::repeat_n("a=1", length), ParseOptions::default()).is_ok(),
        accepted
    );
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(7)]
#[case(31)]
fn generated_assignments_preserve_siblings(#[case] seed: usize) {
    for depth in 1..=8 {
        let mut tokens = vec![PathToken::Key(format!("key/{seed}~\n"))];
        tokens.extend((1..depth).map(|index| {
            if (seed + index) % 2 == 0 {
                PathToken::Index(index % 4)
            } else {
                PathToken::Key(index.to_string())
            }
        }));
        let assignment =
            Jqesque::new(tokens, Some(json!({"value":[seed,null]})), Operation::Merge).unwrap();
        let mut document = json!({"untouched":{"a":[1,2,3]}});
        assignment.apply_to(&mut document).unwrap();
        assert_eq!(document["untouched"], json!({"a":[1,2,3]}));
    }
}

#[test]
fn path_deserialization_stops_before_decoding_excess_tokens() {
    let mut serialized =
        serde_json::to_string(&vec![PathToken::Key("a".into()); DEFAULT_MAX_PATH_DEPTH]).unwrap();
    serialized.pop();
    serialized.push_str(",not-valid-json]");
    let error = serde_json::from_str::<Path>(&serialized).unwrap_err();
    assert!(error.to_string().contains("path depth"), "{error}");
}

#[test]
fn batch_deserialization_stops_before_decoding_excess_assignments() {
    let assignment: Jqesque = "a=1".parse().unwrap();
    let mut serialized =
        serde_json::to_string(&vec![assignment; jqesque::DEFAULT_MAX_BATCH_LENGTH]).unwrap();
    serialized.pop();
    serialized.push_str(",not-valid-json]");
    let error = serde_json::from_str::<Batch>(&serialized).unwrap_err();
    assert!(error.to_string().contains("batch length"), "{error}");
}

#[test]
fn rejected_deeply_constructed_payload_is_dropped_without_recursion() {
    let value = (0..10_000).fold(json!(0), |value, _| Value::Array(vec![value]));
    assert!(matches!(
        Jqesque::from_path(Path::root(), Some(value), Operation::Insert),
        Err(JqesqueError::LimitExceededError {
            kind: LimitKind::ValueDepth,
            ..
        })
    ));
}

#[rstest]
#[case("insert a=1")]
#[case("replace a=1")]
#[case("remove a")]
#[case("merge a=1")]
#[case("merge-patch a=1")]
fn replacing_deep_caller_content_does_not_recurse_on_drop(#[case] input: &str) {
    let value = (0..10_000).fold(json!(0), |value, _| Value::Array(vec![value]));
    let mut document = json!({});
    document["a"] = value;
    input
        .parse::<Jqesque>()
        .unwrap()
        .apply_to(&mut document)
        .unwrap();
    assert!(document.get("a").is_none_or(|value| value == &json!(1)));
}

#[test]
fn batch_input_bytes_are_cumulative() {
    let assignment = format!("a={}", "x".repeat(jqesque::DEFAULT_MAX_INPUT_BYTES / 2));
    let error = Batch::parse([&assignment, &assignment], ParseOptions::default()).unwrap_err();
    assert_eq!(error.index, 1);
    assert!(matches!(
        error.source,
        JqesqueError::LimitExceededError {
            kind: LimitKind::InputBytes,
            ..
        }
    ));
}

#[rstest]
#[case(jqesque::DEFAULT_MAX_INPUT_BYTES, true)]
#[case(jqesque::DEFAULT_MAX_INPUT_BYTES + 1, false)]
fn programmatic_payload_bytes_are_bounded(#[case] bytes: usize, #[case] accepted: bool) {
    let result = Jqesque::from_path(
        Path::root(),
        Some(json!("x".repeat(bytes))),
        Operation::Insert,
    );
    assert_eq!(result.is_ok(), accepted);
}

#[cfg(feature = "arbitrary-precision")]
#[rstest]
#[case(
    "1e99999999999999999999999999999999999999999999",
    "10e99999999999999999999999999999999999999999998",
    true
)]
#[case(
    "1e-99999999999999999999999999999999999999999999",
    "10e-100000000000000000000000000000000000000000000",
    true
)]
#[case("0e99999999999999999999999999999999999999999999", "-0", true)]
#[case("9007199254740993.0", "9007199254740992", false)]
#[case("9007199254740993.0", "9007199254740993", true)]
fn arbitrary_precision_test_equality(
    #[case] actual: &str,
    #[case] expected: &str,
    #[case] equal: bool,
) {
    let mut document = serde_json::from_str(actual).unwrap();
    let assignment: Jqesque = format!("test .={expected}").parse().unwrap();
    assert_eq!(assignment.apply_to(&mut document).is_ok(), equal);
}

#[cfg(feature = "arbitrary-precision")]
#[test]
fn arbitrary_precision_values_survive_assignment_serialization() {
    let assignment: Jqesque = "add x=9007199254740993.00".parse().unwrap();
    let restored: Jqesque =
        serde_json::from_value(serde_json::to_value(&assignment).unwrap()).unwrap();
    assert_eq!(restored, assignment);
}

#[cfg(feature = "arbitrary-precision")]
#[test]
fn fuzz_regression_large_number_round_trip_preserves_meaning() {
    // Minimized from parse_apply: serde_json's Value deserializer may use scientific notation.
    let assignment: Jqesque = "W=77777777777777780000000000000000000000000000000"
        .parse()
        .unwrap();
    let restored: Jqesque =
        serde_json::from_value(serde_json::to_value(&assignment).unwrap()).unwrap();
    assert_eq!(restored, assignment);
    let mut document = json!({});
    restored.apply_to(&mut document).unwrap();
    let test = Jqesque::from_path(
        assignment.path().clone(),
        assignment.value().cloned(),
        Operation::Test,
    )
    .unwrap();
    assert_eq!(test.apply_to(&mut document), Ok(Operation::Test));
}

#[rstest]
#[case("a=1", "a=1.0", true)]
#[case("a=1", "a=2", false)]
#[case("a={\"x\":1}", "a={\"x\":1.0}", true)]
#[case("a=9007199254740993", "a=9007199254740992.0", false)]
fn assignment_equality_uses_json_numeric_semantics(
    #[case] a: &str,
    #[case] b: &str,
    #[case] equal: bool,
) {
    assert_eq!(
        a.parse::<Jqesque>().unwrap() == b.parse::<Jqesque>().unwrap(),
        equal
    );
}

#[test]
fn strict_json_eof_location_points_after_the_input() {
    let input = "payload={\"a\":";
    let result =
        Jqesque::from_str_with_options(input, ParseOptions::default().strict_json_values(true));
    let Err(JqesqueError::InvalidJsonValueError { location, .. }) = result else {
        panic!("expected JSON error");
    };
    assert_eq!(location.offset(), input.len());
    assert_eq!(location.column(), input.len() + 1);
}

#[rstest]
#[case(256, true)]
#[case(257, false)]
fn atomic_clone_depth_boundary(#[case] depth: usize, #[case] accepted: bool) {
    let mut document = (0..depth).fold(json!(0), |value, _| Value::Array(vec![value]));
    let batch = Batch::parse(["replace .=1"], ParseOptions::default()).unwrap();
    let result = batch.apply_atomically(&mut document, ApplyOptions::default());
    assert_eq!(result.is_ok(), accepted);
    if !accepted {
        assert!(matches!(
            result.unwrap_err().source,
            JqesqueError::LimitExceededError {
                kind: LimitKind::DocumentDepth,
                ..
            }
        ));
    }
}

#[rstest]
#[case(0)]
#[case(7)]
#[case(31)]
fn generated_nested_values_round_trip(#[case] seed: usize) {
    for depth in 0..=8 {
        let value = (0..depth).fold(json!([seed, null]), |value, index| {
            if (seed + index) % 2 == 0 {
                json!({"a/~":value})
            } else {
                json!([value])
            }
        });
        let assignment = Jqesque::from_path(Path::root(), Some(value), Operation::Insert).unwrap();
        let restored: Jqesque =
            serde_json::from_value(serde_json::to_value(&assignment).unwrap()).unwrap();
        assert_eq!(restored, assignment);
    }
}

#[test]
fn path_deserialization_checks_cumulative_bytes_before_later_tokens() {
    let tokens = vec![
        PathToken::Key("x".repeat(jqesque::DEFAULT_MAX_INPUT_BYTES / 2)),
        PathToken::Key("y".repeat(jqesque::DEFAULT_MAX_INPUT_BYTES / 2 + 1)),
    ];
    let mut serialized = serde_json::to_string(&tokens).unwrap();
    serialized.pop();
    serialized.push_str(",not-valid-json]");
    let error = serde_json::from_str::<Path>(&serialized).unwrap_err();
    assert!(error.to_string().contains("path bytes"), "{error}");
}

#[cfg(feature = "arbitrary-precision")]
#[rstest]
#[case(
    "-9223372036854775808",
    "-9223372036854775808.0",
    "-9.223372036854776e18"
)]
#[case("9007199254740993", "9007199254740993.0", "9007199254740992.0")]
fn arbitrary_precision_equality_preserves_transitivity(
    #[case] a: &str,
    #[case] b: &str,
    #[case] c: &str,
) {
    let a: Jqesque = format!("a={a}").parse().unwrap();
    let b: Jqesque = format!("a={b}").parse().unwrap();
    let c: Jqesque = format!("a={c}").parse().unwrap();
    assert_eq!(a, b);
    assert_ne!(b, c);
    assert_ne!(a, c);
}
