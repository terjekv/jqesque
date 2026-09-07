# Repository Guidelines

## Verification

- Run `./scripts/check.sh` before considering a change complete. It runs formatting, Clippy, tests, and available
  dependency checks. A skipped security tool is not a successful security check.
- The equivalent required Rust checks are:

  ```bash
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features --locked -- -D warnings
  cargo test --all-features --locked
  cargo test --doc --all-features --locked
  cargo check --manifest-path fuzz/Cargo.toml --bins
  ```

- Rust tests run in parallel. Use focused tests while developing; run the full suite when checking several behaviors.
- Keep formatting mechanical and let rustfmt handle layout.
- Markdown lint must pass for all Markdown files after documentation changes:

  ```bash
  npx markdownlint-cli2 --config .markdownlint.json "**/*.md" "!target" "!fuzz/target"
  ```

- Every fenced code block must declare a language. Use `text` for plain text or ASCII diagrams.
  Keep Markdown tables in one consistent column style, enforced by MD060.
- When adding or moving source, tests, benchmarks, fuzz targets, or other build inputs, check the Cargo manifests,
  `scripts/check.sh`, and `.github/workflows/` together so the affected code is exercised.
- This is a Rust library. Verification does not require a server, `.env`, database, Docker image, or OpenAPI generator.

## Architecture and Type Boundaries

- Treat jqesque as a public library with a supported integration surface. Export deliberate APIs through `src/lib.rs`;
  public types, methods, error behavior, and serialization formats carry compatibility obligations.
- Keep domain types, options, invariants, and public operation behavior in `src/types.rs`.
  Keep syntax recognition and raw input conversion in `src/parse.rs`, and JSON manipulation in `src/manipulators.rs`.
  Split modules by responsibility as they grow; do not introduce application or storage layers without a concrete need.
- Preserve validated facts as types across these boundaries. Convert raw strings, numbers, tokens, and deserialized
  representations once through fallible constructors. Pass the resulting private-fielded domain or proof type onward.
  Downstream code must not reconstruct a validated fact from primitives or repeat validation that the type guarantees.
- Prefer newtypes for scalar invariants such as bounded indices and depths. Use enums for mutually exclusive or
  correlated states, and proof wrappers for parsed or validated state. Encode as much as practical into types so invalid
  combinations cannot be constructed. Apply these rules to new and changed APIs without unrelated wholesale rewrites.
- A type with invariants must have private fields, validating constructors, and explicit read accessors. Mutating APIs
  must preserve those invariants; do not expose unrestricted mutable access, unchecked constructors, or `DerefMut`.
- Implement `TryFrom` or other fallible conversions at untrusted boundaries. Deserialization must use the same
  validation as ordinary construction; deriving `Deserialize` must not silently bypass a newtype's guarantees.
- Keep invariants close to the data they protect. Distinguish syntax validation, resource limits, and checks that depend
  on the target JSON document. Types preserve established facts; runtime-dependent conditions still need runtime checks.
- Keep parser implementation details private. Convert `nom` and JSON Patch errors to crate-owned errors at their
  boundaries, with specific variants and useful context. Never classify an error by searching its display text.
- `serde_json::Value` and Serde traits are intentional integration surfaces. Avoid exposing other third-party
  implementation types in public APIs unless that coupling is deliberate and documented.
- Prefer small, explicit methods and typed options/builders to long positional argument lists. Put behavior in `impl`
  blocks when it belongs to a type. Use typestate only when it prevents meaningful invalid ordering or missing data;
  otherwise use a simple builder whose terminal operation validates its settings.
- Keep core parsing and manipulation deterministic and independent of global configuration, environment variables,
  filesystem access, network calls, and benchmark or fuzzing machinery.

## Rust and API Conventions

- Follow idiomatic Rust and existing repository conventions. Prefer clear implementations over cleverness.
- Accept validated types at boundaries whenever practical, with actionable errors for rejected raw inputs.
- Preserve the documented distinctions between Auto, Insert, Merge, and JSON Patch operations, including null and array
  behavior. Changes to accepted syntax, coercion, resource limits, or mutation semantics need explicit tests and docs.
- Keep permissive parsing and strict JSON parsing deliberate. Maintain constructor and deserialization guarantees so
  infallible methods can rely on them without panicking on externally constructible invalid state.
- Prefer `use` imports over inline fully qualified paths, except for genuine ambiguity or a clearer one-off reference.
- Use conventional Rust module discovery (`foo.rs` or `foo/mod.rs`), not `#[path = "..."]` overrides.
- Keep production dependencies in `[dependencies]`, test and benchmark dependencies in `[dev-dependencies]`, and fuzz
  dependencies in `fuzz/Cargo.toml`. Update lockfiles deliberately and keep CI reproducible with `--locked` where applicable.
- Do not add unused fields, functions, imports, dead code, or warning suppressions to make a build or test pass.

## Tests and Fuzzing

- Put public API behavior tests in `tests/jqesque.rs` and focused implementation tests in their owning modules under
  `#[cfg(test)]`. Exercise behavior through public constructors and accessors rather than bypassing invariants.
- Keep each test focused on one behavior. Use `#[rstest]` and `#[case(...)]` for input variants instead of combining
  unrelated assertions. A small amount of precondition checking for the same behavior is fine.
- Add regression coverage for behavior changes, including strict/permissive parsing, boundary values, malformed paths,
  serialization validation, and resource limits when relevant. Include both accepted boundaries and rejected inputs.
- Test observable results and meaningful error variants. Avoid coupling tests to incidental dependency debug output.
- Keep tests isolated and deterministic under parallel execution; avoid shared mutable global state.
- Maintain the `parse_apply` and `deep_paths_indices` targets in `fuzz/fuzz_targets/` when parser or limit behavior changes.
  Fuzz targets must bound allocations and recursion so they can explore inputs without routine resource exhaustion.
- Check the separate fuzz package when changing it:

  ```bash
  cargo fmt --manifest-path fuzz/Cargo.toml -- --check
  cargo check --manifest-path fuzz/Cargo.toml --bins
  ```

- For parser or hardening changes, run bounded fuzz sessions with a supported nightly toolchain and cargo-fuzz:

  ```bash
  cargo +nightly fuzz run parse_apply -- -max_total_time=60
  cargo +nightly fuzz run deep_paths_indices -- -max_total_time=60
  ```

- Keep generated fuzz artifacts, corpora, and build output out of commits. Minimize a discovered failure and turn it into
  a deterministic regression test when possible.

## Benchmarks

- Put benchmark entrypoints in `benches/` and add matching `[[bench]]` entries with `harness = false` in `Cargo.toml`.
  Keep one benchmark target per file so CI can discover and fan them out independently.
- Use Gungraun for deterministic library benchmarks. Cover parsing, insertion, and merging with representative scalar,
  object, array, sparse-index, and deep-path scenarios. Supply options explicitly; do not read global configuration.
- Keep the benchmark runner version aligned with the library version in `Cargo.toml` and `Cargo.lock`.
  See `CONTRIBUTING.md` for installation and execution commands.
- For harness or dependency changes, run the affected benchmarks with Valgrind in addition to compiling them.
  Keep setup outside the measurement when isolating an operation; include it deliberately for end-to-end scenarios.
- Preserve stable target names when practical so reports remain comparable. Review the measured work when changing a
  benchmark, and do not weaken regression thresholds or silently skip failures to obtain green CI.
- Keep `.github/workflows/bench.yml` compatible with the reusable benchmark workflow. A missing base measurement is
  a new benchmark without a comparison, not evidence that performance improved or stayed constant.

## Dependency Security

- Run `cargo deny check advisories bans sources` and `cargo audit` for dependency or policy changes.
  Install the tools as described in `CONTRIBUTING.md` if they are missing.
- Fix advisories by upgrading or replacing affected dependencies when possible, including development dependencies.
  Do not disable scans, exclude the affected dependency graph, or add broad ignores to make CI pass.
- Any unavoidable advisory exception must identify the advisory, explain the affected dependency and exposure, and state
  what upstream change permits its removal. Keep exceptions narrow and review them on dependency updates.

## Pull Requests and Merges

- Keep each PR focused and independently reviewable. Lead its description with the concrete problem and resulting
  behavior, then explain relevant rationale, compatibility or migration notes, and validation.
- Review `CHANGELOG.md` for every PR. Add user-facing additions, changes, fixes, and security notes to `[Unreleased]`.
  If there is no changelog-worthy impact, say so in the description rather than adding empty or internal-only entries.
- Call out every breaking change in both the PR description and `[Unreleased]`, including the migration users must make.
  Pay particular attention to exported types, field visibility, constructors, accepted inputs, and Serde representations.
- Keep verification in a distinct PR description section so it can be omitted from a squash commit body.
  Include actual results and any validation that could not be completed; do not equate local success with completed CI.
- When squash-merging, use the detailed PR description as the commit body. Preserve substantive summary, rationale,
  behavior notes, and issue references; remove verification-only commands, checklists, and the verification section.
  Follow the user's authorization for publishing and merging.

## Stacked Pull Requests

- Use GitHub stacked PRs for two or more changes with a strict linear dependency, each independently reviewable.
  Put foundational types and shared interfaces below the behavior that depends on them.
- Keep unrelated, merely sequential, fork-based, or branching work in standalone PRs targeting `main`.
  Do not create a stack merely to group independent work or reduce the CI queue.
- Check current GitHub documentation before scripting stack operations; preview features and commands can change.
  Use the official `github/gh-stack` extension, installed with `gh extension install github/gh-stack`.
- For a new stack, use `gh stack init`, `gh stack add`, and `gh stack submit`. Maintain it with `gh stack view`,
  `gh stack sync`, and `gh stack rebase`. Adopt an existing ordered chain with `gh stack link <bottom-pr> ... <top-pr>`.
- Expect required reviews and CI for every layer. Optimize duplicate expensive jobs only after confirming that every
  required check still resolves correctly, including any use of `github.event.pull_request.stack`.
- Merge from the bottom upward with stack-aware tooling, such as `gh stack merge` or the stack-aware asynchronous API.
  Do not use the legacy PR merge API for a stack. Preserve the required squash commit body; pass an explicit title and
  message through the stack-aware API when automation needs exact text.

## Change Discipline

- Keep edits scoped to the requested task. Update code, relevant tests, docs, and verification wiring together.
- Report material limitations honestly. Fix the cause of failing checks without weakening the behavior they protect.
