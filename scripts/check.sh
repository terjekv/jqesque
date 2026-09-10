#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
cargo test --all-features --locked
cargo test --doc --all-features --locked
cargo fmt --manifest-path fuzz/Cargo.toml -- --check
cargo check --manifest-path fuzz/Cargo.toml --bins
cargo +1.85.0 check --lib --all-features --locked
npx --yes markdownlint-cli2@0.20.0 --config .markdownlint.json "**/*.md" "!target" "!fuzz/target"

if command -v cargo-deny >/dev/null 2>&1; then
  cargo deny check advisories bans sources
  cargo deny --manifest-path fuzz/Cargo.toml --config deny.toml check advisories bans sources
else
  echo "cargo-deny is required (cargo install cargo-deny --locked)" >&2
  exit 1
fi

if command -v cargo-audit >/dev/null 2>&1; then
  cargo audit --db target/security/audit
  cargo audit --file fuzz/Cargo.lock --db target/security/audit
else
  echo "cargo-audit is required (cargo install cargo-audit --locked)" >&2
  exit 1
fi
