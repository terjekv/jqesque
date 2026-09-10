# Contributing

Thanks for contributing to jqesque.

## Local quality checks

Run these before opening a PR:

```bash
./scripts/check.sh
```

Equivalent manual commands:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
cargo test --doc --all-features --locked
cargo check --manifest-path fuzz/Cargo.toml --bins
```

## Security checks

Install and run dependency/security checks:

```bash
cargo install cargo-deny --locked
cargo deny check advisories bans sources

cargo install cargo-audit --locked
cargo audit
```

## Fuzzing

Install cargo-fuzz once:

```bash
cargo install cargo-fuzz
```

Run the existing fuzz targets:

```bash
cargo fuzz run parse_apply
cargo fuzz run deep_paths_indices
```

To keep runs bounded locally, use:

```bash
cargo fuzz run deep_paths_indices -- -max_total_time=60
```

## Benchmarks

This repository uses `gungraun` benchmark targets in `benches/`.

Install the runner:

```bash
cargo install gungraun-runner --version 0.19.4 --locked
```

Run selected callgrind benchmarks:

```bash
cargo bench --bench parse_scalar_small_callgrind
cargo bench --bench parse_object_large_callgrind
cargo bench --bench insert_array_sparse_medium_callgrind
cargo bench --bench merge_deep_object_large_callgrind
```

Available benchmark targets are intentionally split by scenario and size to maximize fan-out:

- `parse_scalar_small_callgrind`
- `parse_array_medium_callgrind`
- `parse_deep_path_medium_callgrind`
- `parse_object_large_callgrind`
- `insert_object_small_callgrind`
- `insert_array_sparse_medium_callgrind`
- `insert_deep_path_medium_callgrind`
- `insert_array_dense_large_callgrind`
- `merge_object_small_callgrind`
- `merge_array_medium_callgrind`
- `merge_deep_object_large_callgrind`

CI runs PR benchmarks through the reusable workflow in
[`terjekv/rust-pr-bench`](https://github.com/terjekv/rust-pr-bench).

The workflow pins `v1.3.0` and enables Cargo, runner, and executable caching in the `jqesque-bench` namespace.
Pushes to `main` compile the benchmarks to warm caches that subsequent PRs can restore. PR runs still measure both
revisions and fail on regressions above 3%; only matching builds can reuse cached executables.

Executable reuse assumes reproducible builds. If benchmarks gain external compile-time inputs or runtime assets,
declare them with `binary_cache_key` or `binary_cache_paths` as described in the
[caching guide](https://github.com/terjekv/rust-pr-bench/blob/v1.3.0/docs/caching.md).
