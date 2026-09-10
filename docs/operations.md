# Operation behavior

## Choosing an operation

Use Auto for configuration overrides that may create new paths. Use Replace when a missing path should be an error.
Use Insert when you want deterministic container creation based on path-token types. Use deep Merge to preserve
object members and array tails. Use Merge Patch when interoperating with RFC 7396 deletion and array replacement rules.

## Auto examples

All examples below omit the operator. `auto path=value` has identical behavior.

| Initial document | Assignment | Result | Operation returned |
| --- | --- | --- | --- |
| `{"a":1}` | `a=2` | `{"a":2}` | Replace |
| `{"a":null}` | `a=2` | `{"a":2}` | Replace |
| `{"a":{"x":1,"y":2}}` | `a={"x":3}` | `{"a":{"x":3}}` | Replace |
| `{}` | `a=2` | `{"a":2}` | Add |
| `{}` | `a.b=2` | `{"a":{"b":2}}` | Insert |
| `{"a":1}` | `a.b=2` | `{"a":{"b":2}}` | Insert |
| `{"a":[10,20]}` | `a[1]=99` | `{"a":[10,99]}` | Replace |
| `{"a":[10,20]}` | `a[2]=99` | `{"a":[10,20,99]}` | Add |
| `{"a":[10,20]}` | `a[4]=99` | `{"a":[10,20,null,null,99]}` | Insert |

A failed resource-budget check is an error; it does not change Auto's selected operation or trigger Insert.
Use an explicit operator if replacing incompatible intermediate containers would be undesirable.

## Numeric path components and append

The path token `Key("0")` is distinct from `Index(0)`. Insert, Merge, and Merge Patch create an object for a key
and an array for a bracketed index. For example, `insert a.0=1` produces `{"a":{"0":1}}`, whereas
`insert a[0]=1` produces `{"a":[1]}`.

JSON Patch operations interpret a pointer component according to the existing target container. `add a.0=1`
inserts into an existing array `a`, or writes an object member `"0"` if `a` is an object.
Auto follows those rules in its Replace/Add attempts, then follows token types if it falls back to Insert.

Use `add a.-=1` to append to an existing array. With a slash separator, use `add a/-=1`.
A `-` key is ordinary data for Insert/Merge, and a numeric key is not subject to the bracket-index parsing ceiling.
Target-dependent JSON Patch checks still require an array index to be valid for the actual array length.

## Nulls and merge

Deep Merge treats every explicit null as a value. It can replace an existing non-null value with null, and creates
null members when they do not exist. It does not ignore incoming nulls.

Merge Patch removes null members while processing an object. A null patch applied directly to the selected value
replaces that value with null. To remove a member by name, use Remove or an object patch at its parent.

Deep Merge processes an incoming array by index, including explicit null elements, and retains the target's tail.
Merge Patch replaces arrays wholesale. Neither operation concatenates arrays.

Selecting an array element with the assignment path is distinct from merging an entire array value:
`merge a[2]=9` preserves earlier elements, whereas `merge a=[null,null,9]` explicitly overwrites them with null.

## Root and quoted keys

`.` selects the whole document regardless of the configured separator. `"."` is an ordinary key.
A root Insert, Replace, Add, or Auto can replace the entire document; a root Test compares the whole document.
Root Remove is rejected because a JSON value cannot represent an absent document.

Quoted keys use JSON string escaping. `""`, `"a.b"`, `"a/b"`, `"a~b"`, and `"\u0061"` are valid keys.
Keys beginning with an operator character must be quoted. Unicode identifiers can also be written unquoted.

Full operation names require a separating space or tab. `merge=1` and `merge.child=1` remain ordinary Auto assignments.
When the custom path separator itself is a space or tab, use shorthand operators to avoid an ambiguous named prefix.

## Values and errors

The first optional space after `=` is removed for compatibility. Remaining whitespace is passed to the JSON parser,
or preserved in a fallback string. Leading path whitespace and trailing Remove whitespace are not accepted.
Strict mode requires JSON for the entire right-hand side. Permissive mode converts any JSON parse failure into a string,
including a JSON nesting-limit error; it does not interpret malformed JSON partially.

Test compares JSON data recursively, with numerical equality for numbers. Numeric precision is limited by the
`serde_json::Number` representation used by the application. It does not coerce strings, booleans, or null into numbers.

Each single assignment either succeeds or returns an error without changing the document. Ordered batches retain
successful earlier assignments on failure; atomic batches preserve the original document on any error.
