# CI Pipeline

The GitHub Actions CI pipeline should keep pull-request feedback fast while
making the most important quality and security assertions reproducible. At the
current stage, the pipeline should prefer checks that can run from the Rust
crate and committed repository files without requiring a live ZAP instance or
external service credentials.

## Goals

- Prove that the Rust crate builds, formats, lints, tests, and documents
  cleanly on every pull request.
- Catch dependency, license, and advisory issues before merge.
- Keep the OpenAPI contract and generated documentation from drifting away from
  the implementation.
- Separate fast merge gates from slower scheduled or optional checks.

## Scope

The first version should cover this repository as a Rust service:

- `greenbone-was` binary and library crate.
- Unit and integration-style tests that use in-process fakes, WireMock, and
  SQLite.
- Static API contract files under `doc/`.
- Dependency and supply-chain policy for Cargo dependencies.

It should not require a running ZAP daemon, published container image, or
deployment environment. Live ZAP integration tests are out of scope for this
spec.

## Workflow Structure

Use two required workflow groups:

- `build`: deterministic Rust build, test, lint, documentation, and API-contract
  checks.
- `security`: dependency policy, vulnerability, license, CodeQL, and unused-code
  checks.

Recommended workflow defaults:

- Use the stable Rust toolchain.
- Trigger `build` and lightweight `security` checks on `pull_request` and pushes
  to the default branch.
- Trigger slower security scans on a daily or weekly schedule.
- Use read-only default GitHub token permissions unless a job explicitly needs
  more.
- Cancel superseded pull-request runs for the same branch.
- Cache Cargo registry, Git dependencies, and `target/`, keyed by the lockfile,
  OS, Rust toolchain, and job purpose.
- Treat all checks in this spec as required branch-protection gates for the
  default branch. Reporting jobs such as coverage should still pass when they
  generate and upload their report successfully, even though they do not enforce
  a minimum threshold.
- Enforce `Cargo.lock` with `--locked` on every Cargo command.

## GitHub Actions Policy

The workflow must comply with the repository action allowlist:

- GitHub-owned actions are allowed, for example `actions/checkout`,
  `actions/cache`, `actions/setup-node`, `actions/upload-artifact`, and
  `github/codeql-action`.
- Third-party actions are allowed only when they match the enterprise allowlist
  reported by GitHub policy, for example `codecov/codecov-action@v5`,
  `anchore/scan-action@*`, or the other explicitly listed patterns.
- Do not use unlisted third-party actions such as `dtolnay/rust-toolchain`,
  `Swatinem/rust-cache`, or `taiki-e/install-action`.

For this pipeline, prefer GitHub-owned actions plus shell commands:

- Install and configure Rust with `rustup`, including required components such
  as `rustfmt`, `clippy`, and `llvm-tools-preview`.
- Cache Cargo registry, Git dependencies, and build output with
  `actions/cache`.
- Install Rust CI helper binaries with pinned `cargo install --locked --version`
  commands instead of third-party installer actions.

## Build Jobs

### Format

- Run `cargo fmt --all -- --check`.
- Keep this as a separate, fast-failing job so style failures do not wait behind
  compilation.

### Compile

- Run `cargo check --locked --all-targets`.
- Run `cargo build --locked --bin greenbone-was`.

This catches missing lockfile updates, binary-only failures, test-target
compilation issues, and feature mismatches earlier than tests alone.

### Tests

- Run `cargo test --locked --all-targets`.
- Keep tests hermetic: use WireMock and temporary SQLite databases
  rather than a live ZAP daemon.
- Add `cargo test --locked --doc` once public docs contain executable examples.

`cargo nextest run` can replace `cargo test` later if test runtime becomes a
problem, but the first workflow should avoid adding a new runner unless it
meaningfully improves feedback time.

### Lint

- Run `cargo clippy --locked --all-targets -- -D warnings`.
- Treat warnings as errors in CI so review does not become the enforcement
  mechanism for obvious lint regressions.

### Documentation

- Run `cargo doc --locked --no-deps`.
- Set `RUSTDOCFLAGS="-D warnings"` once the current documentation builds without
  warnings.

### API Contract

- Validate `doc/openapi-reference.yml` as OpenAPI 3.0 YAML.
- Do not add an OpenAPI drift check for now.

This is valuable now because the service depends on an OpenAPI-compatible public
contract and the repository already contains a reference document.

### Binary Smoke Test

- Start `greenbone-was` with the default SQLite storage backend.
- Call `/api/v1/health/ready`, which is the `/health/ready` readiness route
  under the current API base path.
- Use the default port `8030` unless the workflow needs to override the port to
  avoid runner conflicts.
- Fail the job if the readiness endpoint does not return successfully within the
  startup timeout.
- Stop the process cleanly.

## Security Jobs

### Dependency Policy

- Run `cargo deny check`.
- Commit a `deny.toml` as part of the initial CI work.
- Treat the job as required once introduced.
- Include at least these policies:
  - known RustSec advisories,
  - banned duplicate versions where practical,
  - explicitly accepted licenses for direct and transitive dependencies,
  - disallowed git dependencies unless intentionally approved.

### Unused Dependencies

- Run `cargo machete` or an equivalent unused-dependency check.
- Treat unused-dependency findings as required failures. Document intentional
  dependency usage patterns if the tool reports false positives.

### Dependency Updates

- Extend Dependabot from devcontainer-only updates to Cargo updates for `/`.
- Consider a grouped weekly dependency PR for low-risk patch and minor updates.

### CodeQL

- Add GitHub CodeQL for Rust.
- Run CodeQL on every pull request, on pushes to the default branch, and on the
  scheduled security workflow.
- Treat CodeQL as a required pull-request gate from the beginning. There are no
  accepted baseline findings for now.

### Supply-Chain Artifacts

- Generate a CycloneDX SBOM for release builds.
- Upload SBOMs as artifacts for default-branch builds and tagged releases.
- Treat SBOM generation as required for workflows that package or publish the
  service.

## Additional Build Steps Worth Adding Now

The following additions are valuable immediately because they improve signal
without requiring external services:

- `cargo check --locked --all-targets`: currently missing from the draft and
  catches deterministic build and lockfile issues before tests.
- `cargo build --locked --bin greenbone-was`: verifies the shipped binary target,
  not only library and test targets.
- OpenAPI validation for `doc/openapi-reference.yml`: catches broken public API
  documentation early.
- Dependabot Cargo updates: the repository already has Dependabot, but only for
  devcontainers.
- A compiler-based coverage report using Rust's `instrument-coverage` support.
- Markdown lint or link checking for `doc/` once more specs are added.

## Coverage Policy

Coverage should be a required reporting job, but it should not enforce a minimum
coverage threshold:

- Generate line and branch coverage for the Rust crate with Rust compiler
  `instrument-coverage` support.
- Upload the report as a pull-request artifact.
- Pass the job when coverage is generated successfully, regardless of the
  measured percentage.

Coverage is still useful now because the crate already has storage, settings,
HTTP, and ZAP-client tests. A premature threshold would likely create process
noise before the desired test profile is clear.

## Future Checks

These checks are important, but should not block the first CI version:

- Container image build, vulnerability scan, and SBOM attestation.
- API compatibility checks against the upstream scanner API if this service must
  remain wire-compatible.
- Release packaging checks for tags.
- Performance or timeout checks for scan orchestration once long-running scan
  workflows exist.
