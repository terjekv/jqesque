# jqesque

A Rust library to parse simplified JSON assignments in a jq-like syntax and convert them into JSON structures.

Sometimes you want to express simplified JSON assignments as strings without writing the full JSON syntax. This library borrows syntax from [jq](https://jqlang.github.io/jq/) and JSONPath to create a simplified way to represent JSON assignments.

## Features

- **Nested Objects:** Supports nested objects (e.g., `foo.bar.baz=true`).
- **Arrays with Indices:** Supports arrays with indices (e.g., `foo[0].bar=zoot`, where the index must be a positive number).
- **Boolean, Number, and Null Values:** Automatically parses values as booleans, numbers, or null if possible. By default, the value is a string unless serde can parse it as a boolean, number, or null.
- **Custom Separators:** Scopes can be separated by `Separator::Dot` (`.`), `Separator::Slash` (`/`), or `Separator::Custom(char)` (custom character).

Values can be anything that serde_json can parse, including strings, numbers, booleans, null, objects, and arrays.

## Examples

### Basic Usage

```rust
use jqesque::Jqesque;
use serde_json::json;

fn main() {
    let input = "foo.bar[0].baz=hello";
    let jqesque = input.parse::<Jqesque>().unwrap();
    // Without using turbofish syntax:
    // let jqesque: Jqesque = input.parse().unwrap();

    // Alternatively, if you want to specify the separator:
    // let jqesque = Jqesque::from_str_with_separator(input, Separator::Dot).unwrap();

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
use jqesque::{Jqesque, Separator};
use serde_json::json;

fn main() {
    let input = "foo/bar[0]/baz=true";
    let jqesque = Jqesque::from_str_with_separator(input, Separator::Slash).unwrap();

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
use jqesque::{Jqesque, Separator};

let mut json_obj = json!({
    "settings": {
        "theme": {
            "color": "red",
            "font": "Arial",
            "size": 12
        }
    }
});

let input = "settings.theme={\"color\":\"blue\",\"font\":\"Helvetica\"}";
let jqesque = Jqesque::from_str_with_separator(input, Separator::Dot).unwrap();

jqesque.insert_into(&mut json_obj);

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
use jqesque::{Jqesque, Separator};

let mut json_obj = json!({
    "settings": {
        "theme": {
            "color": "red",
            "font": "Arial",
            "size": 12
        }
    }
});

let input = "settings.theme={\"color\":\"blue\",\"font\":\"Helvetica\"}";
let jqesque = Jqesque::from_str_with_separator(input, Separator::Dot).unwrap();

jqesque.merge_into(&mut json_obj);

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

## License

See the [LICENSE](LICENSE) file for details.
