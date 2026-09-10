# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning.

## [Unreleased]

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
