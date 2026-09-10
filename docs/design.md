# Design and integration guarantees

## Public library boundary

jqesque is a supported library API. Exports are deliberate in `src/lib.rs`. Serde and `serde_json::Value` are
intentional
integration surfaces; parser and JSON Pointer implementation types stay private. Public fields, error kinds, syntax,
operation behavior, and serialization formats require compatibility review.

The design follows the relevant principles in
[Hubuum's repository guidelines](https://github.com/hubuum/hubuum/blob/main/AGENTS.md): validate at boundaries, preserve
facts in types, keep invariants near their data, and use small explicit interfaces. jqesque has no application,
database, authorization, container, or server layer.

## Validated types

`PathToken` is a raw representation. `Path` is the proof that a sequence meets global depth, index, and cumulative
sparse-allocation limits. Its fields are private and deserialization validates the same facts. The parser's internal
builder validates incrementally and finishes a Path without reconstructing already established facts.

`Jqesque` stores an internal enum distinguishing Remove from value-bearing operations. Required values cannot be
missing after construction. Explicit JSON null remains distinct from an absent value during deserialization.
A payload wrapper records its validated resource measurements for reuse during application. Parsing measures values
during decoding, and Serde wrappers enforce limits before descending into excess elements or trusting sequence-size
hints. Aggregate batch budgets reuse those measurements.

`Batch` is an ordered, length-bounded collection of validated assignments. Parse and apply options expose builders
and effective limits. An application budget carries remaining capacity through a whole batch.

Target-dependent facts still need runtime checks: whether a parent exists, what container it is, whether an index is
in bounds, and how many slots an operation would allocate. Application checks these conditions before mutation. A
prepared patch target holds an exclusive borrow while its
allocation budget is checked, preserving the resolved target until application.

## Responsibilities

- `src/parse.rs`: recognize assignment syntax, decode quoted keys and raw JSON, and report source locations.
- `src/types.rs` and `src/types/`: domain types, options, invariants, serialization, conversions, and operation
selection.
- `src/manipulators.rs`: target traversal, mutation, merge, and JSON equality.
- `tests/jqesque.rs`: observable behavior through the public API.

Core behavior is deterministic and independent of environment variables, files, network calls, and global configuration.

## Resource boundaries

Input bytes are checked before any tokenization. Path depth and bracketed indices are checked before value decoding.
Potential sparse path allocation is bounded cumulatively; two individually legal large indices cannot allocate two
maximum-sized arrays. Payload depth and node limits cover values supplied through constructors as well as parsing.

Application budgets are conservative: copied payload array slots count even when a merge can reuse existing capacity.
A batch shares one budget, preventing many individually small assignments from bypassing its configured ceiling.
Atomic application additionally clones the caller's existing document. These limits are not a total process-memory
quota and do not include memory already allocated by the caller or Serde's input buffers. Data-byte budgets include
path/object keys, strings, and numeric representations, including large arbitrary-precision numbers.

`as_json()` is an infallible inspection API over a validated assignment. Its preview is bounded by the independent
path and payload proofs, without spending an application budget. Insert/Merge materialize one path and one payload;
Auto materializes one path and three payload copies, plus the small candidate-array wrappers. This deliberately
allows previewing an assignment even when its application would exceed the combined array budget. The explicit
`to_document()` conversion continues to enforce that application budget. Previewing never clones a target document.

Rejected programmatic payloads and replaced document subtrees are disposed of iteratively. Atomic batches and Test
failure diagnostics check caller-owned content before cloning, rejecting depth above 256. This accepts the maximum
depth that a 128-token path plus a 128-level payload can create. An atomic preflight failure is reported at index 0.
Applications remain responsible for how they construct and dispose of their own arbitrary-depth documents.

JSON parsing inherits serde_json's own recursion limit. Permissive parsing intentionally falls back to a string when
JSON parsing fails. Applications that require JSON must enable strict mode. No resource setting raises a global ceiling.

## Verification contract

Run `./scripts/check.sh` for formatting, Clippy, library tests, doctests, fuzz compilation, and dependency scans.
Security checks must run successfully; a missing tool does not establish security. Documentation examples in the README
are also crate doctests, preventing the previous README/rustdoc behavior drift.

CI checks stable Rust and the declared library MSRV. Fuzzing exercises parsing, serialization, application, and limits;
bounded nightly sessions complement deterministic regressions. Benchmarks cover Auto's three outcomes in addition to
parsing, insertion, and merging. Existing benchmark names and regression thresholds remain stable.
