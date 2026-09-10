# Migrating from 0.1.0

This change intentionally revises public APIs and behavior. It requires a breaking release before publication.

## API replacements

| Previous usage | Replacement |
| --- | --- |
| `assignment.value(): &Option<Value>` | `assignment.value(): Option<&Value>` |
| `assignment.operation(): &Operation` | `assignment.operation(): Operation` |
| Matches on `NomError`, `PatchError`, `InvalidPathError` | Match `SyntaxError` or structured `PathError` |
| `InvalidJsonValueError(message)` | `InvalidJsonValueError { location, message }` |
| String matches on `LimitExceededError.kind` | Match the `LimitKind` enum |

`JqesqueError` and error-kind enums are non-exhaustive; downstream matches need a fallback arm.
Operation adds `MergePatch`; update exhaustive matches. It has no shorthand character.
`ParseOptions` limit getters now return the effective, ceiling-clamped settings.

`as_json() -> Value` remains supported for previews and snapshot tests, with its existing output shapes.
Auto still returns `[replace_patch_array, add_patch_array, insert_fragment]`; these are alternatives, not a combined
executable patch. MergePatch adds an explicit `{"op":"merge-patch","path":...,"value":...}` preview descriptor.
Use the additional `to_document()?` or `to_json_patch()?` methods when a specific conversion is required.
Preview allocation uses the validated path and payload bounds independently of application budgets.

## Construction and serialization

Every operation except Remove now requires a value at construction and deserialization. Use `Some(Value::Null)`
for explicit null. Remove rejects supplied values in ordinary construction; use `None`.

Serialized Remove assignments now omit `value`. Legacy Remove records containing `value:null` remain accepted.
Explicit null values in other operations survive round trips. Missing values in value-bearing records are rejected.
Assignment equality now uses numerical JSON equality, including `1` versus `1.0`. Numeric spelling can normalize
through Serde without changing assignment equality or behavior. The JSON Serde object retains its `tokens` and
`operation` field names and existing enum spellings.

## Behavior corrections and new limits

Keys containing `/` or `~` now address their literal JSON keys correctly. If a prior assignment accidentally created
an escaped key such as `a~1b`, migrate that stored document key to its intended spelling before reapplying assignments.
Quoted keys decode JSON escapes correctly, including `\n`; code relying on the old loss of backslashes must use the
intended literal key explicitly.

Indexed Merge now preserves unrelated earlier array elements. Explicit null array elements still overwrite values;
use an array payload such as `merge a=[null,9]` when that is intended. Test accepts numerically equivalent JSON numbers.

Assignments now enforce input-byte, cumulative path-array, payload-depth/node/data-byte, aggregate batch, and
application-array limits.
Reduce or split oversized assignments, choose tighter caller-specific limits where useful, and handle
`LimitExceededError` from application and `to_document()`. Atomic batches and Test diagnostics also reject caller-owned
content deeper than 256 before cloning it. Splitting a batch does not preserve a shared atomic transaction.

The default operation remains Auto. Deep Merge remains `merge` / `~`. RFC 7396 behavior is available only through
`merge-patch`; existing merge expressions do not acquire deletion or whole-array replacement semantics.

## Minimum Rust version

The library declares and verifies Rust 1.85 as its MSRV. Development tooling and fuzzing may require newer toolchains;
see CONTRIBUTING.md. Do not infer a successful CI run from local checks alone.
