# CI Pipeline Implementation Plan

This plan implements the decisions in `doc/specs/ci-pipeline.md` as GitHub
Actions workflows and supporting repository configuration. The first
implementation should stay hermetic: no live ZAP daemon, no deployment
environment, no secret-scanning tool, and no OpenAPI drift generation.

## Target Files

- `.github/workflows/build.yml`
- `.github/workflows/security.yml`
- `.github/dependabot.yml`
- `deny.toml`
- Optional tool configuration files only if a selected tool needs them.

## Phase 0: Action Policy Remediation

Before validating the workflows, remove all third-party actions that are not in
the repository action allowlist.

Replace the currently denied actions as follows:

- Replace `dtolnay/rust-toolchain@stable` with shell steps that run
  `rustup default stable` and, when needed, `rustup component add rustfmt
  clippy llvm-tools-preview`.
- Replace `Swatinem/rust-cache@v2` with GitHub-owned `actions/cache@v4` entries
  for Cargo registry, Cargo git dependencies, and `target/`.
- Replace `taiki-e/install-action@v2` with pinned Cargo installs:
  `cargo install --locked --version 0.6.16 cargo-llvm-cov`,
  `cargo install --locked --version 0.19.6 cargo-deny`, and
  `cargo install --locked --version 0.7.0 cargo-machete`.

Keep the existing GitHub-owned actions:

- `actions/checkout@v4`
- `actions/setup-node@v4`
- `actions/upload-artifact@v4`
- `github/codeql-action/init@v4`
- `github/codeql-action/analyze@v4`

After the replacement, scan the workflows for `uses:` entries and verify that
each one is either GitHub-owned or explicitly included in the enterprise
allowlist reported by GitHub policy.

## Phase 1: Build Workflow

Create `.github/workflows/build.yml` with these defaults:

- Triggers:
  - `pull_request`
  - `push` to the default branch
- Permissions:
  - `contents: read`
- Concurrency:
  - cancel superseded runs for the same workflow and branch or pull request
- Toolchain:
  - stable Rust installed through `rustup`, not a third-party setup action
- Cache:
  - Cargo registry
  - Cargo git dependencies
  - `target/`
  - key includes OS, stable toolchain, `Cargo.lock`, and job purpose
  - implemented with GitHub-owned `actions/cache@v4`

Add required jobs:

- `fmt`
  - `cargo fmt --all -- --check`
- `check`
  - `cargo check --locked --all-targets`
- `build`
  - `cargo build --locked --bin greenbone-was`
- `test`
  - `cargo test --locked --all-targets`
- `clippy`
  - `cargo clippy --locked --all-targets -- -D warnings`
- `doc`
  - `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps`
- `openapi`
  - validate `doc/openapi-reference.yml` as OpenAPI 3.0 YAML
  - do not generate or compare an implementation-backed OpenAPI document
- `smoke`
  - build the binary
  - start `greenbone-was` with the default SQLite backend
  - call `GET http://127.0.0.1:8030/api/v1/health/ready`
  - fail if readiness does not return successfully within the startup timeout
  - always stop the process
- `coverage`
  - use Rust compiler `instrument-coverage` support
  - install `llvm-tools-preview` with `rustup component add`
  - install pinned `cargo-llvm-cov` with `cargo install --locked --version`
  - generate line and branch coverage
  - upload the report as a pull-request artifact
  - do not enforce a coverage threshold

Implementation notes:

- Keep every Cargo command locked with `--locked`.
- Use a dedicated temporary profile/output directory for coverage if needed, so
  coverage instrumentation does not pollute normal build artifacts.
- Prefer a small shell script in the workflow for the smoke test. It should trap
  exit and terminate the background service process.
- If the OpenAPI validator requires a package install, pin the tool version.
- Do not introduce third-party actions unless they are explicitly allowed by the
  enterprise policy list.

## Phase 2: Security Workflow

Create `.github/workflows/security.yml` with these defaults:

- Triggers:
  - `pull_request`
  - `push` to the default branch
  - scheduled daily or weekly run
- Permissions:
  - `contents: read`
  - `security-events: write` only for CodeQL upload
- Concurrency:
  - cancel superseded pull-request runs
- Toolchain:
  - stable Rust installed through `rustup`, not a third-party setup action
- Cache:
  - same Cargo cache strategy as the build workflow

Add required jobs:

- `cargo-deny`
  - install pinned `cargo-deny` with `cargo install --locked --version`
  - run `cargo deny check`
- `unused-dependencies`
  - install pinned `cargo-machete` or the selected equivalent with
    `cargo install --locked --version`
  - run the unused-dependency check
  - fail on findings
- `codeql`
  - use GitHub CodeQL for Rust
  - run on every pull request, pushes to the default branch, and schedule
  - fail on findings because no baseline findings are accepted for now

Do not add secret scanning in this phase.

## Phase 3: Dependency Policy

Add `deny.toml` before enabling the `cargo-deny` job as required.

The initial policy should cover:

- RustSec advisories for direct and transitive dependencies
- accepted licenses for direct and transitive dependencies
- duplicate dependency versions where practical
- disallowed git dependencies unless explicitly approved

Implementation steps:

1. Generate or write the first `deny.toml`.
2. Run `cargo deny check` locally.
3. Tighten license allowlists until the result is intentional.
4. Document any unavoidable exception in `deny.toml`.

## Phase 4: Dependabot

Update `.github/dependabot.yml`:

- Keep existing devcontainer updates.
- Add Cargo updates for `/`.
- Use a weekly schedule.
- Group low-risk patch and minor Cargo updates if the repository policy allows
  grouped updates.

## Phase 5: Supply-Chain Artifacts

Add CycloneDX SBOM generation only to workflows that package or publish the
service.

Initial implementation:

- If there is no packaging or release workflow yet, do not add an SBOM job to
  pull-request CI.
- When packaging is introduced, generate a CycloneDX SBOM and upload it as a
  workflow artifact.
- Make SBOM generation required for package and publish workflows.

## Phase 6: Branch Protection

After the workflows have run successfully once, configure default-branch
protection to require every check introduced by this spec:

- build workflow jobs
- security workflow jobs
- coverage report generation
- CodeQL

Coverage remains required only as report generation. It must not require a
minimum percentage.

## Validation

Before opening the implementation pull request:

- Run `cargo fmt --all -- --check`.
- Run `cargo check --locked --all-targets`.
- Run `cargo build --locked --bin greenbone-was`.
- Run `cargo test --locked --all-targets`.
- Run `cargo clippy --locked --all-targets -- -D warnings`.
- Run `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps`.
- Run `cargo deny check`.
- Run the selected unused-dependency check.
- Run the OpenAPI validator against `doc/openapi-reference.yml`.
- Manually verify the smoke-test command against
  `http://127.0.0.1:8030/api/v1/health/ready`.

## Non-Goals

- Do not require a live ZAP daemon.
- Do not add live-ZAP integration tests.
- Do not add secret scanning.
- Do not add OpenAPI drift checking.
- Do not enforce a coverage threshold.
- Do not add container build or image scanning until packaging exists.
