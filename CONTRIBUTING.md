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
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --doc --all-features
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

This repository uses `iai-callgrind` benchmark targets in `benches/`.

Install the runner:

```bash
cargo install cargo-iai-callgrind
```

Run selected callgrind benchmarks:

```bash
cargo iai-callgrind --bench parse_scalar_small_callgrind
cargo iai-callgrind --bench parse_object_large_callgrind
cargo iai-callgrind --bench insert_array_sparse_medium_callgrind
cargo iai-callgrind --bench merge_deep_object_large_callgrind
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
`terjekv/github-action-iai-callgrind`.
