# Scan Module Implementation Plan

This plan implements the behavior defined in `doc/specs/scan-module.md` with a new internal scan orchestration layer that coordinates API commands, storage state, and ZAP client operations.

## Target Files

- `src/scan/mod.rs` (new)
- `src/scan/status.rs` (new)
- `src/scan/queue.rs` (new)
- `src/scan/worker.rs` (new)
- `src/scan/progress.rs` (new)
- `src/scan/retry.rs` (new)
- `src/scan/errors.rs` (new)
- `src/lib.rs`
- `src/app/mod.rs`
- `src/api/scans.rs`
- `src/api/dto/scans.rs` (remove lifecycle enum; keep transport DTOs only)
- `src/storage/interface.rs`
- `src/storage/sqlite.rs`
- `src/storage/sqlite_tests.rs`
- `src/config/settings.rs`

## Phase 0: State Model and Contract Alignment

Status: Done (2026-06-08)

First align the scan-domain lifecycle status model and command semantics with the spec.

- Add scan-domain `ScanStatus` in the scan module (for example `src/scan/status.rs`) and replace current lifecycle variants with:
  - `new`
  - `queued`
  - `running`
  - `stop requested`
  - `stopped`
  - `interrupted`
  - `done`
- Remove lifecycle status ownership from API DTOs and make API/storage depend on the scan-domain `ScanStatus` type.
- Introduce one transition validator in the scan module and remove duplicated state logic from API handlers.
- Enforce non-idempotent `start_scan` and `stop_scan` behavior through this validator.
- Keep API response mapping in `src/api/scans.rs`, but move transition decisions and lifecycle typing into scan domain code.

Phase 0 amendments (2026-06-08):

- Implemented the transition validator directly on `ScanStatus` in `src/scan/status.rs` as methods (`start_command_transition`, `stop_command_transition`, `can_delete`) instead of a separate free-function validator in `src/scan/mod.rs`.
- Updated API handlers to call the scan-domain status methods for transition and delete checks.
- Moved transition unit tests from `src/scan/mod_tests.rs` to `src/scan/status_tests.rs` and attached them to the status sidecar test module.

## Phase 1: Scan Service Facade

Create an internal scan service interface used by API handlers.

- Add a `ScanService` (or equivalent module-level orchestration API) with commands:
  - `create_scan`
  - `start_scan`
  - `stop_scan`
  - `delete_scan`
  - `get_results`
- Re-export scan-domain `ScanStatus` from the scan module and use it everywhere scan lifecycle state is persisted or serialized (including API responses).
- Return typed domain errors:
  - `InvalidTransition`
  - `ScanNotFound`
  - `InvalidUrl`
  - wrapped storage and zap client errors
- Ensure `create_scan` persists scan in `new` and does not enqueue automatically.

## Phase 2: Storage Model Extensions and Atomic Updates

Add persistence support for runtime worker state and safe transitions.

- Extend persisted scan record fields to include:
  - context name
  - context id
  - queue timestamp
  - run start timestamp
  - terminal timestamp
  - alert cursor / processed alert count
  - serialized progress payload
  - optional interruption reason
- Add storage methods for:
  - atomic transition update with expected previous scan-domain lifecycle status
  - progress update
  - context metadata update
  - alert cursor update
  - listing scans in non-terminal states (startup recovery)
- Use SQL transactions for all state + progress changes done together.
- Keep existing results storage behavior and retain partial results for stopped/interrupted scans.

## Phase 3: Queue and Worker Runtime

Implement asynchronous execution with FIFO queue and configurable worker count.

- Add FIFO queue abstraction with:
  - enqueue by scan id
  - dequeue for worker
  - remove queued scan by id (for queued stop)
- Add worker supervisor with default single worker and configurable max workers.
- Worker execution pipeline per scan:
  1. transition `queued` -> `running`
  2. create/reuse context `greenbone-was-{scan_uuid}`
  3. include escaped target regex patterns in context
  4. run AJAX spider per target and update stage progress
  5. run active scan per target and update stage progress
  6. poll alerts while scanning using pagination cursor
  7. cleanup context and finalize `done`
- On worker/internal error in non-terminal states, transition to `interrupted`.
- If cleanup fails after successful scan completion, keep scan lifecycle status `done` and log warning.

## Phase 4: Stop Flow and Interruption Rules

Implement strict stop semantics for queued and running scans.

- `queued` + stop:
  - remove from queue
  - transition directly to `stopped`
- `running` + stop:
  - transition to `stop requested`
  - signal worker cancellation
  - worker performs graceful stop and transitions to `stopped`
- Add configurable stop grace period (default 5 minutes):
  - if exceeded, force stop and transition to `interrupted`
- If ZAP stop actions fail non-transiently, transition to `interrupted`.

## Phase 5: URL Validation and Retry Backoff

Implement spec-compliant target validation and resilient external calls.

- Validate comma-separated target URLs:
  - absolute HTTP/HTTPS only
  - trim surrounding whitespace
  - reject user-info
  - reject fragments
  - reject whitespace/control characters in URL
  - reject dot patterns as defined by spec
- Return `InvalidUrl` with original value + reason on validation failure.
- Add retry helper with exponential backoff:
  - start delay 1 second
  - configurable max retries (default 10)
  - configurable max delay (default 60 seconds)
- Use retry for transient storage lock/contention and transient ZAP/network failures.
- Exhausted retries transition active scan to `interrupted`.

## Phase 6: Progress Model and Alerts Polling

Implement persisted progress and percentage calculations.

- Track per target:
  - spider stage state: pending/running/done
  - spider last state from ZAP: running/stopped
  - active scan stage state: pending/running/done
  - active scan percentage 0..100
- Compute per-target overall percentage:
  - `25` if spider is done plus `0.75 * active_scan_percent`
  - floor to integer
- Aggregate target progress to scan-level percentage.
- Poll alerts at configurable interval (default 10 seconds).
- Use pagination start offset from persisted processed-alert count to avoid duplicates.

## Phase 7: Startup Recovery and Wiring

Wire scan runtime into service startup.

- In `src/lib.rs` startup path:
  - initialize scan service + queue + worker supervisor
  - run recovery pass: all non-terminal scans become `interrupted`
  - start worker tasks before serving API traffic
- Extend `AppState` to include scan service handle and resolved scan-runtime configuration values used by handlers/workers.
- Keep API module as transport boundary only (HTTP parsing + response mapping).

## Phase 8: Observability and Telemetry

Add required logs/telemetry from the spec.

- Emit info logs + telemetry on every scan lifecycle status transition.
- Emit queue wait time telemetry (`queued` -> `running`).
- Log transient ZAP/storage failures as warnings when retries remain.
- Log retry exhaustion as error before transitioning to `interrupted`.

## Phase 9: Tests

Follow repository sidecar test pattern.

- Add unit tests for all valid transitions.
- Add unit tests for invalid transitions (must error).
- Add worker-path tests for:
  - successful completion to `done`
  - queued stop path to `stopped`
  - running stop path through `stop requested` to `stopped`
  - forced stop timeout to `interrupted`
  - retry exhaustion to `interrupted`
- Add startup recovery test:
  - non-terminal scans become `interrupted` on startup.
- Add URL validation tests for all rejection rules.
- Add alert pagination tests ensuring duplicate avoidance.

## Configuration Additions

Configuration ownership and flow:

- Define scan-related configuration keys and defaults in `src/config/settings.rs`.
- Load and validate settings in the config module, then pass resolved values into global app state during startup.
- Consume current runtime values from global app state in scan service/worker code instead of re-reading environment variables.

Add settings with defaults (defined in `src/config/settings.rs`):

- `scan_worker_count = 1`
- `scan_alert_poll_interval_seconds = 10`
- `scan_retry_max_retries = 10`
- `scan_retry_max_delay_seconds = 60`
- `scan_stop_grace_period_seconds = 300`

## Validation Checklist

Before opening the implementation PR:

- `cargo fmt --all -- --check`
- `cargo check --locked --all-targets`
- `cargo test --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- Confirm there is a single canonical scan-domain lifecycle status enum in scan module and no duplicate lifecycle enum in API DTOs.
- Manual API checks:
  - create -> `new`
  - start (`new`) -> `queued`
  - worker pick -> `running`
  - stop queued -> `stopped`
  - stop running -> `stop requested` -> `stopped`
  - runtime failure in non-terminal -> `interrupted`
  - done/stopped/interrupted/new deletable

## Non-Goals

- No scan prioritization policy beyond FIFO in this phase.
- No advanced worker resource scheduling (RAM/CPU-based) in this phase.
- No multi-instance distributed queue coordination in this phase.
- No switch to per-scan dedicated ZAP instance in this phase.