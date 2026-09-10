# jqesque

A Rust library for applying small, readable JSON assignments. Use it for configuration overrides,
CLI arguments, and programmatic JSON updates.

```rust
use jqesque::{Jqesque, Operation};
use serde_json::json;

let mut document = json!({"settings": {"theme": "light"}});
let assignment: Jqesque = "settings.theme=dark".parse().unwrap();
assert_eq!(assignment.apply_to(&mut document).unwrap(), Operation::Replace);
assert_eq!(document, json!({"settings": {"theme": "dark"}}));
```

Rust **1.85 or newer** is required for the library. Development checks use current stable Rust;
fuzzing uses nightly. Enable the optional `arbitrary-precision` feature to preserve decimal JSON numbers beyond
64-bit precision. The integration surfaces are crate-owned types, Serde, and `serde_json::Value`.

## Syntax

An assignment is an optional operator, a path, `=`, and a value. Remove has no `=` or value.
You can use either a shorthand or a lowercase operation name followed by spaces or tabs:

```text
settings.theme=dark
insert settings.theme="dark"
>settings.theme="dark"
merge settings={"font":"mono"}
~settings={"font":"mono"}
remove settings.obsolete
-settings.obsolete
merge-patch settings={"obsolete":null}
```

Full names are `auto`, `insert`, `merge`, `merge-patch`, `add`, `remove`, `replace`, and `test`.
Names are case-sensitive. `insert=1` still assigns to a key named `insert`.

Paths use dot separators by default: `items[0].name`. Slash and custom separators are available through
`ParseOptions`. Quoted keys follow JSON string rules, including empty keys, escaped quotes, Unicode escapes,
and literal separators: `"a.b".""=1`. Indices in brackets are nonnegative integers.
The path `.` selects the whole document; `"."` selects a key literally named `.`.

## What happens without an operator?

**No operator means Auto. Auto never merges.** It selects the first applicable operation:

1. **Replace** when the complete path exists. The selected value is overwritten, including objects and arrays.
2. **Add** when the parent exists and accepts the final key or index. Array index `len` appends.
3. **Insert** otherwise. Missing containers are created, incompatible intermediate values are replaced,
   and sparse arrays are padded with nulls.

```rust
use jqesque::{Jqesque, Operation};
use serde_json::json;

let mut document = json!({"settings": {"theme": "light", "font": "mono"}});
let assignment: Jqesque = "settings={\"theme\":\"dark\"}".parse().unwrap();
assert_eq!(assignment.apply_to(&mut document).unwrap(), Operation::Replace);
assert_eq!(document, json!({"settings": {"theme": "dark"}}));
// The old font is gone. Use "merge settings=..." to keep it.
```

The return value reports the concrete operation performed. Resource-limit failures are returned to the caller;
they do not trigger another Auto fallback.

## Operations

| Name | Shorthand | Existing target | Missing target or parent |
| --- | --- | --- | --- |
| `auto` | none | Replace the selected value | Add if possible, otherwise Insert |
| `insert` | `>` | Overwrite the selected value | Create containers and pad sparse arrays |
| `merge` | `~` | Deep merge objects and arrays by index | Create containers and pad sparse arrays |
| `merge-patch` | name only | Apply RFC 7396 to the selected value | Create containers, then apply RFC 7396 |
| `add` | `+` | Overwrite an object member; insert before an array element | Parent must exist; array index may equal its length |
| `replace` | `=` | Overwrite the selected value | Error |
| `remove` | `-` | Delete the member or element; shift later array elements left | Error |
| `test` | `?` | Compare without mutation | Error |

Add, Replace, Remove, and Test use [JSON Patch semantics (RFC 6902)](https://www.rfc-editor.org/rfc/rfc6902).
Remove requires a member or element; removing the whole document is rejected. To set the whole document to null,
use `replace .=null`. Append with `add items.-=value` or, using a slash separator, `add items/-=value`.
Bracket syntax `[-]` is not accepted.

## Deep merge and JSON Merge Patch

`merge` / `~` is jqesque's deep merge. `merge-patch` explicitly selects
[JSON Merge Patch (RFC 7396)](https://www.rfc-editor.org/rfc/rfc7396).

| Incoming data | `merge` / `~` | `merge-patch` |
| --- | --- | --- |
| Object | Recursively merge members | Recursively merge members |
| Null object member | Store null as a value | Delete that member |
| Array | Merge by index and preserve the existing tail | Replace the entire array |
| Scalar, or null as the selected value | Replace the selected value | Replace the selected value |
| Object applied to a scalar | Replace with the object, keeping its null members | Start an empty object and apply deletion rules |

For `merge-patch settings=null`, the selected `settings` value becomes null. To delete the `settings` member,
use `remove settings` or `merge-patch .={"settings":null}`. Selecting a nested value with a path is a jqesque extension;
RFC 7396 itself operates on an entire document.

```rust
use jqesque::Jqesque;
use serde_json::json;

let original = json!({"settings": {"old": true, "items": [1, 2]}});
let mut deep = original.clone();
let mut rfc = original;
"merge settings={\"old\":null,\"items\":[9]}"
    .parse::<Jqesque>().unwrap().apply_to(&mut deep).unwrap();
"merge-patch settings={\"old\":null,\"items\":[9]}"
    .parse::<Jqesque>().unwrap().apply_to(&mut rfc).unwrap();
assert_eq!(deep, json!({"settings": {"old": null, "items": [9, 2]}}));
assert_eq!(rfc, json!({"settings": {"items": [9]}}));
```

An indexed deep merge such as `merge items[2].enabled=true` changes only that selected element.
Existing elements at indices 0 and 1 are preserved.

## Parsing untrusted input

Permissive parsing tries JSON first and falls back to a string. Thus `enabled=true` contains a boolean,
`enabled="true"` contains a string, and `enabled=tru` contains the string `tru`.
Use strict mode to reject malformed JSON values. Empty strings must be quoted: `name=""`.

```rust
use jqesque::{ApplyOptions, Jqesque, ParseOptions, Separator};
use serde_json::json;

let options = ParseOptions::new(Separator::Dot)
    .strict_json_values(true)
    .max_input_bytes(16_384)
    .max_path_depth(32)
    .max_array_index(1_024);
let assignment = Jqesque::from_str_with_options("items[2]=true", options).unwrap();
let mut document = json!({});
assignment.apply_to_with_options(
    &mut document,
    ApplyOptions::new().max_array_slots(4_096),
).unwrap();
```

Limits have exported global ceilings. Setters tighten them and accessors report effective limits:

| Resource | Default / global ceiling | When checked |
| --- | --- | --- |
| Assignment input bytes | 1,048,576 | Before tokenization; also the aggregate input ceiling for a batch |
| Path tokens | 128 | During tokenization, before value decoding |
| Bracketed array index | 1,000,000 | During tokenization, before value decoding |
| Potential array slots along a path | 1,000,001 | Path construction and tokenization |
| Payload depth | 128 edges from the value root | Assignment construction |
| Payload nodes | 1,000,000 | Construction and incremental deserialization; shared across a batch |
| Decoded path/payload data bytes | 1,048,576 | Construction and incremental deserialization; shared across a batch |
| Atomic/Test diagnostic document depth | 256 edges | Before cloning caller-owned content |
| Application array slots | 1,000,001 | Before mutation; shared across a batch |
| Batch length | 1,024 assignments | Construction and parsing |

Application budgets conservatively count newly created path array slots plus array slots in copied payloads.
They bound this work, rather than measuring every allocator byte. They do not count the caller's existing document,
including atomic batch clones and Test failure diagnostics. Decoded data counts path/object keys, strings, and number
representations; it is not serialized JSON size. A caller or deserializer may already have allocated input before
validation. See the [design guide](docs/design.md) for boundaries.

Errors expose crate-owned kinds. Syntax errors include a zero-based UTF-8 byte offset and one-based line and
character column. Path errors identify the JSON Pointer and zero-based token index. Strict JSON errors retain a
parser message and a location in the complete assignment.

## Programmatic construction and serialization

`Path` validates raw tokens once and can be reused with `Jqesque::from_path`. Its fields are private.
`Jqesque::new` remains a convenience constructor taking raw tokens, an optional value, and an operation.
Every operation except Remove requires a value; use `Some(Value::Null)` for an explicit null.

`Jqesque` serializes with `tokens`, `operation`, and a `value` member for value-bearing operations.
Remove omits `value`; the legacy Remove encoding with `value:null` is still accepted.
Deserialization uses the same operation/value and path validation as construction. Assignments compare values using
JSON numerical equality. Serde can normalize a number's spelling, such as a large integer into equivalent scientific
notation; round trips preserve its numerical meaning rather than its original text.

There are two explicit conversions:

- `to_document()` materializes Insert or deep Merge as a JSON fragment. This is data, not an RFC merge patch.
  For indexed paths, merging the fragment elsewhere may not reproduce `apply_to`.
- `to_json_patch()` exports Add, Remove, Replace, or Test as an RFC 6902 patch array.

Unsupported conversions return an error. Auto needs a target document to choose its operation and cannot be exported
as a target-independent JSON Patch. Serialize an assignment with Serde when you want to store and restore its intent.

## Multiple assignments

`Batch::parse` parses all assignments before application. `apply_to` applies them in order and retains earlier
successful changes if a later assignment fails. `apply_atomically` commits only after every assignment succeeds.
Both report the zero-based index and original error for a failed assignment, and share an application budget.

```rust
use jqesque::{ApplyOptions, Batch, ParseOptions};
use serde_json::json;

let batch = Batch::parse(["settings.theme=dark", "?settings.enabled=true"], ParseOptions::default()).unwrap();
let mut document = json!({"settings": {"theme": "light", "enabled": false}});
let original = document.clone();
let error = batch.apply_atomically(&mut document, ApplyOptions::default()).unwrap_err();
assert_eq!(error.index, 1);
assert_eq!(document, original);
```

## Further documentation

- [Operation examples and edge cases](docs/operations.md)
- [Architecture, guarantees, and limits](docs/design.md)
- [Migration guide](docs/migration.md)
- [Correctness, performance, and security review](docs/review.md)
- [Changelog](CHANGELOG.md)
- [Contributing, verification, fuzzing, and benchmarks](CONTRIBUTING.md)
- [Release instructions](.github/RELEASING.md)

## License

[MIT](LICENSE).
