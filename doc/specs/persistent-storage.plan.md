# Persistent Storage Default Plan

This plan changes the default WAS SQLite storage from ephemeral in-memory
storage to a persistent database under the Greenbone WAS variable data
directory. In-memory SQLite remains available only for the SQLite storage
module's own single-threaded unit tests.

## Target Files

- `src/config/settings.rs`
- `src/config/settings_tests.rs`
- `src/lib.rs`
- `src/storage/sqlite.rs`
- `src/storage/sqlite_tests.rs`
- `src/storage/mod.rs`
- `src/storage/test_support.rs` (new)
- `src/scan/service_tests.rs`
- `src/scan/worker_tests.rs`
- `src/scan/state_coordinator/mod_tests.rs`
- `src/scan/state_coordinator/transition_executor_tests.rs`
- `src/http/router_tests.rs`
- `Cargo.toml`
- `doc/specs/scan-module.md`
- `README.md` (optional)

## Phase 1: Variable Data Directory Setting

Status: Done (2026-06-16)

Add a configurable variable data directory and derive the default database URL
from it.

- Add a `var_data_dir` setting in `src/config/settings.rs`, exposed through the
  environment variable `GREENBONE_WAS_VAR_DATA_DIR`.
- Define `DEFAULT_VAR_DATA_DIR` as `/var/lib/greenbone-was`.
- Define the default database filename separately, for example `scans.db`.
- Derive the default SQLite URL from `var_data_dir` plus the default database
  filename instead of hard-coding a database in the current working directory;
  with defaults, the derived URL is `sqlite:/var/lib/greenbone-was/scans.db`.
- Treat `GREENBONE_WAS_SQLITE_URL` as an explicit override: when it is set, use
  it as-is after validation; when it is unset, build the SQLite URL from
  `var_data_dir`.
- Remove or stop exporting `SQLITE_IN_MEMORY_URL` from the public settings API.
- Keep `StorageBackend::Sqlite` as the only runtime storage backend.
- Update `src/config/settings_tests.rs` to assert the variable data directory
  default and the derived persistent SQLite default.

## Phase 2: Runtime Data Directory Preparation

Status: Done (2026-06-16)

Create the configured variable data directory before opening the default
database there.

- Update `src/lib.rs` startup to create `settings.var_data_dir` before
  initializing SQLite when the service uses the derived default database URL.
- Map directory creation failures to an application startup error with enough
  context to diagnose permission or filesystem problems.
- If `GREENBONE_WAS_SQLITE_URL` is set explicitly, do not create unrelated
  parent directories unless that is intentionally handled by SQLite URL path
  parsing.
- Add or adjust startup-focused coverage if a local pattern for testing `run()`
  setup exists; otherwise cover the decision in settings tests and leave full
  startup behavior to smoke testing.

## Phase 3: Reject In-Memory Runtime Configuration

Reject in-memory SQLite URLs through normal application configuration.

- Add an in-memory SQLite URL detector in `src/config/settings.rs`.
- Reject `sqlite::memory:` and obvious `mode=memory` forms in
  `Settings::from_raw` with a clear error that points users to file-backed
  SQLite URLs.
- Add settings coverage for `GREENBONE_WAS_SQLITE_URL=sqlite::memory:`.
- Keep the existing explicit file URL override test.
- Add settings coverage proving `GREENBONE_WAS_VAR_DATA_DIR` changes the
  derived default database URL when `GREENBONE_WAS_SQLITE_URL` is unset.
- Add settings coverage proving explicit `GREENBONE_WAS_SQLITE_URL` wins over
  `GREENBONE_WAS_VAR_DATA_DIR`.

## Phase 4: Enforce Constructor Policy

Make the storage constructor itself enforce the same policy so direct callers do
not accidentally create runtime in-memory databases.

- Add an `is_in_memory_sqlite_url` helper in `src/storage/sqlite.rs`.
- Make public `SqliteStorage::new(url)` reject in-memory URLs with
  `StorageError::Backend` before opening the pool.
- Refactor construction into a private internal helper, for example
  `new_with_in_memory_policy(url, allow_in_memory)`.
- Use the private helper only from storage unit tests to intentionally allow
  in-memory SQLite there.
- Update constructor documentation to use persistent examples and describe
  in-memory SQLite as storage-test-only.

## Phase 5: Single-Threaded Storage Unit Tests

Keep in-memory SQLite only in `src/storage/sqlite_tests.rs`, and make that
exception explicit.

- Define the `sqlite::memory:` test URL locally in `src/storage/sqlite_tests.rs`.
- Change the storage test fixture to call the private in-memory-allowed
  constructor path.
- Annotate every SQLite storage test with
  `#[tokio::test(flavor = "current_thread")]`.
- Add coverage proving public `SqliteStorage::new("sqlite::memory:")` rejects
  the URL while the storage test helper can still construct the in-memory
  database.

## Phase 6: Persistent Temporary Test Databases

Move every non-storage test away from in-memory SQLite.

- Add `tempfile` as a dev-dependency in `Cargo.toml`.
- Add `#[cfg(test)] pub(crate) mod test_support;` to `src/storage/mod.rs`.
- Add `src/storage/test_support.rs` with a helper that:
  - creates a `TempDir`,
  - builds a unique `sqlite:<tempdir>/scans.db` URL,
  - opens `SqliteStorage::new`,
  - returns both the storage handle and the `TempDir` guard.
- Do not add any in-memory helper to shared test support.

## Phase 7: Migrate Non-Storage Tests

Replace imports of `SQLITE_IN_MEMORY_URL` and direct in-memory storage creation
outside `src/storage/sqlite_tests.rs`.

- Update `src/scan/service_tests.rs` to use the temporary file-backed helper.
- Update `src/scan/worker_tests.rs` to use the helper, keeping the temp
  directory guard alive while worker tasks run.
- Update `src/scan/state_coordinator/mod_tests.rs` to use the helper.
- Update `src/scan/state_coordinator/transition_executor_tests.rs` to use the
  helper.
- Update `src/http/router_tests.rs` to use the helper.

## Phase 8: Documentation Cleanup

Update stale test-storage wording.

- Update `doc/specs/scan-module.md` so scan-module tests refer to temporary
  file-backed SQLite storage.
- Note that storage module unit tests are the only allowed in-memory SQLite
  exception, and that those tests run on a single-thread Tokio runtime.
- Optionally add a short storage configuration note to `README.md`, documenting
  `GREENBONE_WAS_VAR_DATA_DIR`, the default database filename under that
  directory, and `GREENBONE_WAS_SQLITE_URL` as an explicit override.

## Verification

- Run `cargo fmt --all -- --check`.
- Run `cargo test --locked storage::sqlite_tests -- --test-threads=1`.
- Run `cargo test --locked config::settings_tests` to verify the variable data
  directory default, the derived default database URL, explicit
  `GREENBONE_WAS_SQLITE_URL` override behavior, and runtime in-memory URL
  rejection.
- Run targeted migrated tests:

  ```sh
  cargo test --locked scan::service_tests scan::state_coordinator::mod_tests scan::state_coordinator::transition_executor_tests http::router_tests
  ```

- Run worker runtime tests:

  ```sh
  cargo test --locked scan::worker_tests
  ```

- Run `cargo test --locked --all-targets`.
- Run `cargo clippy --locked --all-targets -- -D warnings`.
- Run the final policy sweep:

  ```sh
  rg "SQLITE_IN_MEMORY_URL|sqlite::memory:" src doc Cargo.toml .github
  ```

Expected result: no shared config constant, no non-storage test uses in-memory
SQLite, and `sqlite::memory:` appears only in storage tests or explicit policy
tests/messages.

## Decisions

- Use `/var/lib/greenbone-was` as the Greenbone WAS default variable data
  directory, with `scans.db` as the default database filename.
- Keep `GREENBONE_WAS_SQLITE_URL` as an explicit override for deployments that
  need a custom SQLite URL.
- Treat single-threaded storage tests as
  `#[tokio::test(flavor = "current_thread")]` in `src/storage/sqlite_tests.rs`
  only.
- Reject in-memory SQLite through both settings validation and public
  `SqliteStorage::new` so accidental runtime or direct use fails clearly.
- Use temporary file-backed SQLite databases for service, worker, state
  coordinator, and router tests.
- Do not reintroduce an in-memory storage backend or change storage schema.