# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning.

## [Unreleased]

### Added

- Introduced parse hardening via ParseOptions with strict JSON mode and configurable path depth and array index limits.
- Added dedicated error variants for strict JSON failures and limit validation failures.
- Added CI workflows for formatting, linting, tests, doc tests, dependency policy checks, and advisory scans.
- Added fuzzing harness and targets for parser/apply paths and deep path/index stress scenarios.
- Added benchmark infrastructure with iai-callgrind and scenario-specific fan-out benchmark targets.
- Added PR benchmark workflow integration using terjekv/github-action-iai-callgrind.
- Added CONTRIBUTING guide and local quality/security helper script.

### Changed

- Migrated parameterized tests from yare to rstest.
- Encapsulated Jqesque internals with constructor/getter APIs and limit validation on apply paths.
- Reduced clone churn in insert path manipulation by passing values by reference in recursion.
- Updated documentation and examples to prefer options-based parsing for untrusted input.

### Security

- Strengthened protections against resource-exhaustion style inputs by enforcing parse/apply safety limits.
