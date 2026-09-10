# Publishing jqesque

The `Publish to crates.io` workflow uses
[crates.io trusted publishing](https://crates.io/docs/trusted-publishing) to authenticate with GitHub OIDC.
It publishes when a `v<version>` tag is pushed, after testing the tagged commit and verifying the Cargo package.
Pull requests run the tests and publish dry run without publishing credentials.

## One-time setup

Before pushing the first release tag:

1. Create the `crates-io` environment in the
   [GitHub repository settings](https://github.com/terjekv/jqesque/settings/environments).
   Restrict deployments to tags matching `v*` and configure required reviewers if desired.
2. As a crate owner, open [jqesque settings on crates.io](https://crates.io/crates/jqesque/settings),
   then add a GitHub trusted publisher with these exact values:

   | Field | Value |
   | --- | --- |
   | Repository owner | `terjekv` |
   | Repository name | `jqesque` |
   | Workflow filename | `publish.yml` |
   | Environment | `crates-io` |

The workflow filename excludes `.github/workflows/`. The environment name must match on both services.
These settings live outside the repository and must be configured separately from merging the workflow.
No crates.io API token secret is needed. The authentication action obtains a temporary token immediately
before publishing and revokes it when the job finishes.

## Release a version

1. Update the package version in `Cargo.toml` and `Cargo.lock` to a version not yet published on crates.io.
   Prepare the release notes and merge the release changes into `main` through a reviewed PR.
2. Update a clean local checkout of `main` and verify the package:

   ```bash
   git switch main
   git pull --ff-only origin main
   cargo test --all-features --locked
   cargo publish --dry-run --locked --registry crates-io
   ```

3. Tag that commit with `v` followed by the exact version from `Cargo.toml`, then push only that tag.
   For example, when releasing version `0.1.0`:

   ```bash
   git tag -a v0.1.0 -m "Release v0.1.0"
   git push origin v0.1.0
   ```

4. Watch the [publishing workflow](https://github.com/terjekv/jqesque/actions/workflows/publish.yml)
   and approve the environment deployment if required. Confirm the new version appears on crates.io.

The workflow rejects tags that do not match the manifest version or whose commit is not on `main`.
Only a tag push in `terjekv/jqesque` can enter the publishing job, which is the only job with OIDC permission.
Concurrent publishing jobs are serialized, and a newer run does not cancel an in-progress upload.

If a run fails before uploading, fix the configuration and rerun the failed jobs for the same tag.
If the upload succeeded, the published version cannot be overwritten; use a new version for subsequent changes.
