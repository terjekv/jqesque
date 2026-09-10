# Correctness, performance, and security review

## Scope and method

Reviewed parsing, constructors, Serde, operation selection, path resolution, mutation, numerical equality,
resource accounting, public exports, docs, and verification wiring against the 0.1.0 implementation at `74543b7`.
The review combined source inspection, public API reproductions, deterministic generated cases, RFC examples,
Valgrind instruction measurements, and bounded fuzzing. The changes form one library/API improvement PR.

## Findings addressed

| Area | Finding | Resulting behavior |
| --- | --- | --- |
| Pointer conversion | Keys containing `/` and `~` were escaped twice | Each raw key is escaped once |
| Serde | Explicit null became a missing value | Null and absence remain distinct; invalid operations are rejected at construction |
| Indexed deep merge | Temporary null padding erased earlier array elements | Merge operates directly at the selected value |
| Quoted keys | Escapes lost meaning; empty keys and Unicode escapes failed | JSON string decoding and located syntax errors |
| Test | Integer/floating comparisons used representation equality | Numerical equality without integer-to-float rounding or loss of transitivity |
| Numeric round trips | Serde can normalize a large integer into scientific notation | Assignment equality follows JSON numerical meaning, with a fuzz regression |
| Auto | Repeated attempts cloned paths and payloads | Select behavior through target checks, then perform one mutation |
| Mutation | Runtime facts were discarded between checking and applying | Prepared targets hold an exclusive borrow until application |
| Limits | Individual path/index bounds did not limit aggregate growth | Cumulative path, payload, input, and batch budgets |
| Deserialization | Collections could allocate before aggregate validation | Incremental checks and ignored speculative sequence hints |
| Stack safety | Rejected or replaced deep values used recursive disposal | Iterative disposal and guarded diagnostic/atomic cloning |
| Documentation | README and rustdoc disagreed about operations | Shared README doctests plus explicit operation and migration guides |
| Verification | Missing security tools could be skipped | Required scans fail closed; root and fuzz graphs are checked |

The design borrows the relevant principles from
[Hubuum's guidelines](https://github.com/hubuum/hubuum/blob/main/AGENTS.md): preserve validated facts in private types,
use enums for correlated states, keep dependency details behind crate-owned boundaries, and verify observable behavior.

## Merge decision

Keep `merge` / `~` as deep merge: null is a stored value, arrays merge by index, and unspecified content survives.
Add `merge-patch` as an explicit RFC 7396 operation: null object members delete keys and arrays replace wholesale.
Applying RFC rules to a selected subtree is a jqesque extension; root `merge-patch .=...` operates on the whole document.
This avoids silently changing existing expressions into deletion requests. The RFC Appendix A examples are regression
cases, alongside tests showing the difference from deep merge.

## Performance measurements

Both revisions were compiled on the same machine with the same stable toolchain and Valgrind. Existing benchmark
sources were unchanged. The three new Auto benchmark files were also copied into the archived base checkout, so their
comparisons have actual base measurements. Auto setup is outside the measured operation; assignment disposal is included.
These are instruction counts, not wall-clock latency claims or completed GitHub CI results.

| Benchmark case | Base instructions | New instructions | Change |
| --- | --- | --- | --- |
| `auto_add_object_medium / scalar` | 10,463 | 5,035 | -51.9% |
| `auto_add_object_medium / object` | 160,457 | 30,183 | -81.2% |
| `auto_insert_deep_path_medium / scalar` | 21,727 | 9,318 | -57.1% |
| `auto_insert_deep_path_medium / object` | 222,562 | 34,767 | -84.4% |
| `auto_replace_object_medium / scalar` | 5,526 | 3,290 | -40.5% |
| `auto_replace_object_medium / object` | 80,972 | 28,091 | -65.3% |
| `insert_array_dense_large` | 21,981 | 20,727 | -5.7% |
| `insert_array_sparse_medium` | 17,312 | 17,751 | +2.5% |
| `insert_deep_path_medium` | 18,369 | 15,929 | -13.3% |
| `insert_object_small` | 7,758 | 7,300 | -5.9% |
| `merge_array_medium` | 8,266 | 6,517 | -21.2% |
| `merge_deep_object_large` | 34,479 | 24,249 | -29.7% |
| `merge_object_small` | 14,552 | 9,726 | -33.2% |
| `parse_array_medium` | 12,206 | 12,021 | -1.5% |
| `parse_deep_path_medium` | 14,865 | 10,875 | -26.8% |
| `parse_object_large` | 26,482 | 22,860 | -13.7% |
| `parse_scalar_small` | 1,744 | 1,232 | -29.4% |

Validation initially added overhead to array parsing. Measuring budgets during decoding, avoiding numeric formatting
allocations, and reusing the node-count proof for array-slot bounds removed that regression. Regression thresholds and
existing benchmark workloads were preserved. The production graph now has four direct dependencies: Serde, serde_json,
jsonptr, and thiserror.

## Verification and practical boundaries

The full check script covers default/all-feature tests, doctests, formatting, Clippy, MSRV 1.85, fuzz compilation,
Markdown lint, and both root/fuzz dependency graphs with cargo-deny and cargo-audit. The integration suite includes
constructor/deserialization boundaries, rollback, generated nested values, 10,000-level disposal cases, arbitrary
precision, and RFC behavior.

Both fuzz targets are run for bounded 60-second campaigns with nightly and cargo-fuzz. The parser campaign found the
large-number serialization case described above; a deterministic regression now checks its numerical meaning and
application behavior. A first sandboxed campaign could not complete LeakSanitizer's process inspection; verification
uses an environment that permits the sanitizer checks. Advisory scans use local locks because the host Cargo home
is on NFS, while the two scanners retain separate advisory database checkouts.

Resource budgets are conservative work limits, not a process-memory quota. Atomic application still clones an existing
document. A single caller-supplied string or deserializer input buffer may already be allocated before the library can
reject it. Arbitrary-precision number spelling can normalize through Serde; numerical meaning is preserved. Auto still
creates/replaces incompatible intermediate containers, and deep merge still overwrites explicit nulls. These behaviors
are documented rather than silently changed. Longer fuzz campaigns can explore additional inputs beyond this review.

This PR contains breaking API, error, serialization, equality, and resource-limit changes. See the
[migration guide](migration.md) and [Unreleased changelog](../CHANGELOG.md) before publishing a breaking release.
