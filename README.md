# jqesque

A Rust library to parse simplified JSON assignments in a jq-like syntax and convert them into JSON structures.

Sometimes you want to express simplified JSON assignments as strings without writing the full JSON syntax. This library borrows syntax from [jq](https://jqlang.github.io/jq/) and JSONPath to create a simplified way to represent JSON assignments.

## Features

- **Nested Objects:** Supports nested objects (e.g., `foo.bar.baz=true`).
- **Arrays with Indices:** Supports arrays with indices (e.g., `foo[0].bar=zoot`, where the index must be a positive number).
- **Boolean, Number, and Null Values:** Automatically parses values as booleans, numbers, or null if possible. By default, the value is a string unless serde can parse it as a boolean, number, or null.
- **Custom Separators:** Scopes can be separated by `Separator::Dot` (`.`), `Separator::Slash` (`/`), or `Separator::Custom(char)` (custom character).

Values can be anything that serde_json can parse, including strings, numbers, booleans, null, objects, and arrays.

## Syntax

The syntax is inspired by [jq](https://jqlang.github.io/jq/) and JSONPath [RFC9535](https://datatracker.ietf.org/doc/html/rfc9535) and is as follows:

```text
[<operation>]<path>=[<value>]
```

- `<operation>`: An optional operation to perform. Supported operations are Add (+), Replace (=), Remove (-), Test (?), Insert (>), and Merge (~).
- `<path>`: The path to the JSON key. The path can be nested and can include array indices. The path can be separated by a dot (`.`), a slash (`/`), or a custom character.
- `<value>`: A JSON value. Note that the Remove operation does not require a value.

### Operations

Add, Remove, Replace, and Test operations are done as per the JSON Patch specification in [RFC6902](https://datatracker.ietf.org/doc/html/rfc6902/).

- **Add (+):** Adds a value to an object or inserts it into an array. In the case of an array, the value is inserted before the given index. The - character can be used instead of an index to insert at the end of an array.
- **Remove (-):** Removes the key or element from the JSON structure.
- **Replace (=):** Replaces the value of an existing key. If the key does not exist, the operation fails. Equivalent to a “remove” followed by an “add”.
- **Test (?):** Tests if the key-value pair exists in the JSON structure.
- **Insert (>):** Inserts a new key-value pair into the JSON structure. If the key already exists, the operation overwrites the value.
- **Merge (~):** Preforms a deep merge of the value into the existing JSON structure. null values are preserved in the existing structure. Note that this behavior **differs** from from JSON Merge Patch defined in [RFC7396](https://datatracker.ietf.org/doc/html/rfc7396).

For more information, see the Operation enum itself.

### Paths

Paths can be nested and can include array indices. The path can be separated by a dot (`.`), a slash (`/`), or a custom character.

### Values

Values are parsed by serde_json. The library will attempt to parse the value as a JSON value, defaulting to string.

## Safety and ParseOptions

When parsing untrusted input (e.g. from HTTP requests, CLI arguments, or config files), use
`ParseOptions` to bound resource usage and enforce strict value parsing.

- `strict_json_values(true)`: require the right-hand side to be valid JSON.
- `max_path_depth(n)`: reject deeply nested paths.
- `max_array_index(n)`: reject oversized array indices.

The default limits are exported as `DEFAULT_MAX_PATH_DEPTH` and `DEFAULT_MAX_ARRAY_INDEX`.

### API selection guide

| API | Best for | Notes |
| --- | --- | --- |
| `input.parse::<Jqesque>()` | Quick defaults with dot separator | Uses permissive value parsing and default limits. |
| `Jqesque::from_str_with_separator(input, separator)` | Convenience with custom separator | Prefer for trusted/simple input. |
| `Jqesque::from_str_with_options(input, options)` | Untrusted input and production boundaries | Recommended: supports strict parsing and custom limits. |

### Recommended setup for untrusted input

```rust
use jqesque::{Jqesque, ParseOptions, Separator};

let options = ParseOptions::new(Separator::Dot)
    .strict_json_values(true)
    .max_path_depth(64)
    .max_array_index(10_000);

let jqesque = Jqesque::from_str_with_options("settings.theme=\"dark\"", options)?;
```

If you prefer permissive parsing (legacy behavior), keep `strict_json_values(false)`.

## Examples

### Basic Usage

```rust
use jqesque::Jqesque;
use serde_json::json;

fn main() {
    let input = ">foo.bar[0].baz=hello";
    let jqesque = input.parse::<Jqesque>().unwrap();
    // Without using turbofish syntax:
    // let jqesque: Jqesque = input.parse().unwrap();

    // Recommended for untrusted input:
    // let options = ParseOptions::new(Separator::Dot)
    //     .strict_json_values(true)
    //     .max_path_depth(64)
    //     .max_array_index(10_000);
    // let jqesque = Jqesque::from_str_with_options(input, options).unwrap();

    let json_output = jqesque.as_json();
    assert_eq!(json_output, json!({
        "foo": {
            "bar": [
                {
                    "baz": "hello"
                }
            ]
        }
    }));
}
```

### Specifying the separator

```rust
use jqesque::{Jqesque, ParseOptions, Separator};
use serde_json::json;

fn main() {
    let input = ">foo/bar[0]/baz=true";
    let options = ParseOptions::new(Separator::Slash)
        .strict_json_values(true)
        .max_path_depth(64)
        .max_array_index(10_000);
    let jqesque = Jqesque::from_str_with_options(input, options).unwrap();

    let json_output = jqesque.as_json();
    assert_eq!(json_output, json!({
        "foo": {
            "bar": [
                {
                    "baz": true
                }
            ]
        }
    }));
}
```

### Inserting into an existing JSON structure

```rust
use serde_json::json;
use jqesque::{Jqesque, ParseOptions, Separator};

let mut json_obj = json!({
    "settings": {
        "theme": {
            "color": "red",
            "font": "Arial",
            "size": 12
        }
    }
});

let input = ">settings.theme={\"color\":\"blue\",\"font\":\"Helvetica\"}";
let options = ParseOptions::new(Separator::Dot)
    .strict_json_values(true)
    .max_path_depth(64)
    .max_array_index(10_000);
let jqesque = Jqesque::from_str_with_options(input, options).unwrap();

jqesque.apply_to(&mut json_obj);

let expected = json!({
    "settings": {
        "theme": {
            "color": "blue",
            "font": "Helvetica"
        }
    }
});

assert_eq!(json_obj, expected);
// Note that the "size" key in the original "theme" object is removed.
```

### Merging into an existing JSON structure

```rust
use serde_json::json;
use jqesque::{Jqesque, ParseOptions, Separator};

let mut json_obj = json!({
    "settings": {
        "theme": {
            "color": "red",
            "font": "Arial",
            "size": 12
        }
    }
});

let input = "~settings.theme={\"color\":\"blue\",\"font\":\"Helvetica\"}";
let options = ParseOptions::new(Separator::Dot)
    .strict_json_values(true)
    .max_path_depth(64)
    .max_array_index(10_000);
let jqesque = Jqesque::from_str_with_options(input, options).unwrap();

jqesque.apply_to(&mut json_obj);

let expected = json!({
    "settings": {
        "theme": {
            "color": "blue",
            "font": "Helvetica",
            "size": 12
        }
    }
});

assert_eq!(json_obj, expected);
// Note that the "size" key in the original "theme" object is preserved.
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for local quality checks, dependency/security checks,
and fuzzing commands.

## Benchmarking

Benchmarks are split into small, file-scoped targets in `benches/` to improve fan-out and
parallel CI execution.

- `benches/parse_scalar_small_callgrind.rs`
- `benches/parse_array_medium_callgrind.rs`
- `benches/parse_deep_path_medium_callgrind.rs`
- `benches/parse_object_large_callgrind.rs`
- `benches/insert_object_small_callgrind.rs`
- `benches/insert_array_sparse_medium_callgrind.rs`
- `benches/insert_deep_path_medium_callgrind.rs`
- `benches/insert_array_dense_large_callgrind.rs`
- `benches/merge_object_small_callgrind.rs`
- `benches/merge_array_medium_callgrind.rs`
- `benches/merge_deep_object_large_callgrind.rs`

Install the runner and run selected benchmark targets:

```bash
cargo install cargo-iai-callgrind
cargo iai-callgrind --bench parse_scalar_small_callgrind
cargo iai-callgrind --bench parse_object_large_callgrind
cargo iai-callgrind --bench insert_array_sparse_medium_callgrind
cargo iai-callgrind --bench merge_deep_object_large_callgrind
```

PR benchmark reporting and regression gating uses
`terjekv/github-action-iai-callgrind` via `.github/workflows/bench.yml`.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release notes and notable changes.

## License

See the [LICENSE](LICENSE) file for details.
