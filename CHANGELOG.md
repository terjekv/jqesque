# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning.

## [Unreleased]

### Added

- Full operation names alongside shorthands: `auto`, `insert`, `merge`, `add`, `remove`, `replace`, and `test`.
- Explicit `merge-patch` operation with RFC 7396 semantics at a selected value, and `.` root-path syntax.
- Reusable validated `Path`, bounded ordered `Batch`, atomic batch application, and `ApplyOptions`.
- Structured syntax locations and path-error kinds, with assignment indices for batch failures.
- Separate `to_document()` and `to_json_patch()` conversions with errors for unsupported operations.
- Documented `as_json()` previews for visual and snapshot testing, preserving the existing signature and output
  shapes. MergePatch previews expose an explicit operation/path/value descriptor.
- Optional `arbitrary-precision` feature and numerical Test equality across integer and decimal representations.
- Detailed Auto, merge, operation-edge-case, architecture, and migration documentation; README examples are doctests.
- Auto benchmarks for all three fallback outcomes, library MSRV checks, Markdown CI, and scheduled bounded fuzzing.

### Changed

- **Breaking:** Assignment equality compares numbers numerically, so `1` and `1.0` are equivalent. Do not use assignment
  equality to detect changes in numeric spelling; Serde may normalize equivalent numeric representations.
- **Breaking:** `value()` now returns `Option<&Value>` and `operation()` returns the Copy enum `Operation` by value.
- **Breaking:** Match structured `SyntaxError` and `PathError` instead of `NomError`, `PatchError`, and
`InvalidPathError`.
  `InvalidJsonValueError` now has `location` and `message` fields; `LimitExceededError.kind` is a `LimitKind` enum.
  Error enums are non-exhaustive: add fallback match arms. Handle the new `Operation::MergePatch` variant.
- **Breaking:** Constructors and deserialization reject missing values except for Remove. Use `Some(Value::Null)` for
  explicit null and `None` for Remove. Remove serialization omits `value`; legacy Remove `value:null` remains accepted.
- **Breaking:** New input, decoded-data, payload-depth/node, cumulative sparse-path, batch, and application budgets
  reject oversized assignments. Reduce inputs or split work and handle limit errors. Option getters report clamped
limits.
- **Breaking:** Atomic application and Test failure diagnostics reject caller-owned content deeper than 256 before
  cloning it. Reduce document depth; an atomic preflight failure is reported at assignment index 0.
- Declare Rust 1.85 as the supported library MSRV; development tooling and fuzzing use newer toolchains.
- Replace cloned Auto attempts with target checks and prepared mutation guards. Remove the nom, nom-language, and
  json-patch production dependencies; keep parser and pointer implementation details private.
- Required local verification now fails if a security tool is missing and includes Markdown and MSRV checks.

### Fixed

- JSON Pointer keys containing `/` or `~` are escaped exactly once. Migrate any accidentally escaped stored keys to
their
  intended literal spelling before reapplying affected assignments.
- Explicit JSON null survives assignment serialization and remains distinct from a missing value.
- Indexed deep Merge preserves earlier array elements. To overwrite them with null, supply explicit null array entries.
- Quoted keys use JSON string decoding, including empty keys, Unicode escapes, and control-character escapes.
  Inputs relying on the former loss of escape backslashes must use their intended literal keys explicitly.
- Test compares numbers numerically, including nested values, without rounding neighboring large integers together.
- Auto budget errors do not trigger a different mutation fallback; all single-assignment errors precede mutation.

### Security

- Check input size and array indices before value decoding; enforce aggregate path allocation and batch budgets.
- Bound payload decoding and ignore untrusted sequence allocation hints. Reject excess path tokens and batch entries
  before decoding them, preserving validated measurements through parsing, construction, and deserialization.
- Dispose of rejected deeply nested programmatic payloads and replaced subtrees iteratively to avoid recursive Drop.
- Check numeric exponent normalization without machine-integer overflow, including arbitrary-precision representations.

## [0.1.0] - 2026-09-10

### Added

- Introduced parse hardening via ParseOptions with strict JSON mode and configurable path depth and array index limits.
- Added dedicated error variants for strict JSON failures and limit validation failures.
- Added CI workflows for formatting, linting, tests, doc tests, dependency policy checks, and advisory scans.
- Added fuzzing harness and targets for parser/apply paths and deep path/index stress scenarios.
- Added benchmark infrastructure with Gungraun and scenario-specific fan-out benchmark targets.
- Added PR benchmark workflow integration using `terjekv/rust-pr-bench@v1.3.0`, with parallel target discovery
  and a 3% regression gate.
- Cache benchmark dependencies, build outputs, runners, and compatible executables, with build-cache warming on `main`.
- Added CONTRIBUTING guide and local quality/security helper script.

### Changed

- **Breaking:** `Jqesque` fields are private. Replace struct literals with `Jqesque::new(tokens, value, operation)?`
  and field reads with `tokens()`, `value()`, and `operation()`.
- **Breaking:** Parsing and deserialization now reject paths deeper than 128 tokens or indices above 1,000,000.
  Custom options can tighten these ceilings. Split oversized assignments or reduce indices before upgrading.
- **Breaking:** `JqesqueError` adds `InvalidJsonValueError` and `LimitExceededError`. Update exhaustive matches
  to handle these variants.
- Migrated parameterized tests from yare to rstest.
- Encapsulated Jqesque internals with constructor/getter APIs and limit validation on apply paths.
- Reduced clone churn in insert path manipulation by passing values by reference in recursion.
- Updated documentation and examples to prefer options-based parsing for untrusted input.

### Fixed

- Constrained jsonptr to the version family used by json-patch so fresh dependency resolution and fuzz builds compile.
- Kept cargo-deny and cargo-audit database directories separate so the sequential CI security checks can both run.
- Strict JSON parsing now returns `InvalidJsonValueError` with the JSON parser's diagnostic, while malformed assignment
  syntax still returns `NomError`.

### Security

- Enforce effective path depth limits during tokenization, before allocating excess keys or indices or decoding values.
  Parsing stops at the first excess token, reporting `found` as `limit + 1` without inspecting the remaining input.
- Strengthened protections against resource-exhaustion style inputs by enforcing parse/apply safety limits.
- Enforced path invariants at construction and deserialization so `as_json()` cannot bypass resource limits.
- Replaced the obsolete iai-callgrind dependency chain with Gungraun and removed the bincode advisory exception.
