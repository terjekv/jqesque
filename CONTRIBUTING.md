# Contributing

Thanks for contributing to jqesque.

## Local quality checks

Run these before opening a PR:

```bash
./scripts/check.sh
```

Equivalent manual commands:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
cargo test --all-features --locked
cargo test --doc --all-features --locked
cargo fmt --manifest-path fuzz/Cargo.toml -- --check
cargo check --manifest-path fuzz/Cargo.toml --bins
cargo +1.85.0 check --lib --all-features --locked
npx --yes markdownlint-cli2@0.20.0 --config .markdownlint.json "**/*.md" "!target" "!fuzz/target"
```

Install the MSRV toolchain with `rustup toolchain install 1.85.0 --profile minimal`. Node.js 22 or newer is used
for the pinned Markdown linter. The helper fails when a required security tool is missing.

## Security checks

Install and run dependency/security checks:

```bash
cargo install cargo-deny --locked
cargo deny check advisories bans sources
cargo deny --manifest-path fuzz/Cargo.toml --config deny.toml check advisories bans sources

cargo install cargo-audit --locked
cargo audit --db target/security/audit
cargo audit --file fuzz/Cargo.lock --db target/security/audit
```

The tools use separate advisory database caches under `target/security/`. Keeping these on the project filesystem
avoids advisory database lock failures when the home directory is on NFS. No advisories are excluded.

## Fuzzing

Install cargo-fuzz once:

```bash
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --locked
```

Run the existing fuzz targets:

```bash
cargo +nightly fuzz run parse_apply
cargo +nightly fuzz run deep_paths_indices
```

Run both targets with bounded sessions after parser or hardening changes:

```bash
cargo +nightly fuzz run parse_apply -- -max_total_time=60
cargo +nightly fuzz run deep_paths_indices -- -max_total_time=60
```

A scheduled workflow runs both targets for 60 seconds and retains failure artifacts. Local sanitizer checks require
an environment that permits LeakSanitizer process inspection; a sandbox shutdown failure is not a successful campaign.

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
- `auto_replace_object_medium_callgrind`
- `auto_add_object_medium_callgrind`
- `auto_insert_deep_path_medium_callgrind`

The Auto benchmarks prepare scalar/object payloads outside measurement and measure application plus assignment
disposal. For a local before/after comparison, run identical benchmark sources against both revisions with the same
toolchain and Valgrind. A first measurement with no base is not evidence of improvement.

CI runs PR benchmarks through the reusable workflow in
[`terjekv/rust-pr-bench`](https://github.com/terjekv/rust-pr-bench).

The workflow pins `v1.3.0` and enables Cargo, runner, and executable caching in the `jqesque-bench` namespace.
Pushes to `main` compile the benchmarks to warm caches that subsequent PRs can restore. PR runs still measure both
revisions and fail on regressions above 3%; only matching builds can reuse cached executables.

Executable reuse assumes reproducible builds. If benchmarks gain external compile-time inputs or runtime assets,
declare them with `binary_cache_key` or `binary_cache_paths` as described in the
[caching guide](https://github.com/terjekv/rust-pr-bench/blob/v1.3.0/docs/caching.md).
