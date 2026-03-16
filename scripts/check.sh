#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --doc --all-features

if command -v cargo-deny >/dev/null 2>&1; then
  cargo deny check advisories bans sources
else
  echo "cargo-deny not installed; skipping (cargo install cargo-deny --locked)"
fi

if command -v cargo-audit >/dev/null 2>&1; then
  cargo audit
else
  echo "cargo-audit not installed; skipping (cargo install cargo-audit --locked)"
fi
