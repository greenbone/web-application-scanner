# Scan Module Implementation Plan

This plan implements the behavior defined in `doc/specs/scan-module.md` with a new internal scan orchestration layer that coordinates API commands, storage state, and ZAP client operations.

## Target Files

- `src/scan/mod.rs` (new)
- `src/scan/scan.rs` (new, or equivalent domain model file)
- `src/scan/status.rs` (new)
- `src/scan/queue.rs` (new)
- `src/scan/worker.rs` (new)
- `src/scan/scan_state_coordinator.rs` (new)
- `src/scan/state_coordinator/mod.rs` (new coordinator module)
- `src/scan/state_coordinator/execution_state_executor.rs` (new)
- `src/scan/state_coordinator/transition_executor.rs` (new)
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
  - `stored`
  - `requested`
  - `running`
  - `stopped`
  - `failed`
  - `succeeded`
- Represent running stop intent with a separate scan-model boolean flag (`stop_requested`) instead of a dedicated lifecycle status.
- Remove lifecycle status ownership from API DTOs and make API/storage depend on the scan-domain `ScanStatus` type.
- Introduce one transition validator in the scan module and remove duplicated state logic from API handlers.
- Enforce non-idempotent `start_scan` and `stop_scan` behavior through this validator.
- Keep API response mapping in `src/api/scans.rs`, but move transition decisions and lifecycle typing into scan domain code.

Phase 0 amendments (2026-06-08):

- Implemented the transition validator directly on `ScanStatus` in `src/scan/status.rs` as methods (`start_command_transition`, `stop_command_transition`, `can_delete`) instead of a separate free-function validator in `src/scan/mod.rs`.
- Updated API handlers to call the scan-domain status methods for transition and delete checks.
- Moved transition unit tests from `src/scan/mod_tests.rs` to `src/scan/status_tests.rs` and attached them to the status sidecar test module.

## Phase 1: Scan Service Facade

Status: Done (2026-06-08)

Create an internal scan service interface used by API handlers.

- Add a `ScanService` (or equivalent module-level orchestration API) with commands:
  - `get_default_preferences`
  - `create_scan`
  - `start_scan`
  - `stop_scan`
  - `delete_scan`
  - `get_scan`
  - `get_scan_status`
  - `get_result`
  - `get_results`
- Re-export scan-domain `ScanStatus` from the scan module and use it everywhere scan lifecycle state is persisted or serialized (including API responses).
- Return typed domain errors:
  - `InvalidTransition`
  - `ScanNotFound`
  - `InvalidUrl`
  - wrapped storage and zap client errors
- Ensure `create_scan` persists scan in `stored` and does not enqueue automatically.
- Keep `HEAD /scans` metadata as a transport-only API concern.

Phase 1 amendments (2026-06-08):

- Initial service facade was wired for `create_scan`, `start_scan`, `stop_scan`, `delete_scan`, and `get_results`.
- `GET /scans/preferences` is served via `ScanService::get_default_preferences` and currently returns default/static preferences.
- Remaining read endpoints (`get_scan`, `get_scan_status`, `get_scan_result`) were migrated to `ScanService` so API handlers no longer call storage directly.
- Read-command error mapping is unified through scan-domain service errors while API responsibilities remain limited to HTTP parsing and response mapping.

## Phase 2: Storage Model Extensions and Atomic Updates

Status: Done (2026-06-09)

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
  - batch alert-to-result persistence (multiple alerts in one transaction)
  - alert cursor update (executed only after successful alert-to-result batch commit)
  - listing scans in non-terminal states (startup recovery)
- Use SQL transactions for all state + progress changes done together.
- Keep existing results storage behavior and retain partial results for stopped/failed scans.

Phase 2 amendments (2026-06-08):

- Extended `ScanRecord` persistence model with context metadata, queue/runtime timestamps, alert cursor, progress payload, and interruption reason.
- Added storage operations for compare-and-swap status transitions, progress updates, context metadata updates, alert cursor updates, batch result persistence, and listing non-terminal scans.
- SQLite schema migrations for newly added scan columns are out of scope for now.
- Phase 2 assumes a fresh SQLite database, or manual schema recreation, when these columns are introduced.
- Implemented batch result writes (`add_results`) in a single SQL transaction and retained existing single-result behavior via delegation.

## Phase 3: Queue and Worker Runtime

Status: Done (2026-06-09)

Implement asynchronous execution with FIFO queue and configurable worker count.

- Add FIFO queue abstraction with:
  - enqueue by scan id
  - dequeue for worker
  - remove requested scan by id (for requested stop)
- Add worker supervisor with default single worker and configurable max workers.
- Worker execution pipeline per scan:
  1. transition `requested` -> `running`
  2. create/reuse context `greenbone-was-{scan_uuid}`
  3. include escaped target regex patterns in context
  4. run AJAX spider per target and update stage progress
  5. run active scan per target and update stage progress
  6. poll alerts while scanning using pagination cursor
  6.1 convert fetched alert batches to result records via transactional multi-alert storage function
  6.2 update the alert cursor only after the batch persistence transaction succeeds
  7. cleanup context and finalize `succeeded`
- On worker/internal error in non-terminal states, transition to `failed`.
- If cleanup fails after successful scan completion, keep scan lifecycle status `succeeded` and log warning.

## Phase 3A: Observability and Telemetry (Unblocked)
Status: Done (2026-06-10)

Pull forward observability work that does not depend on unfinished phases.

- Emit info logs on every scan lifecycle status transition.
- Emit info logs for scan creation and scan deletion commands.
- Emit queue wait time telemetry (`requested` -> `running`).

## Phase 3B: Scan Domain Type and Scan State Coordinator Boundaries

Status: Done (2026-06-10)

Introduce a scan-domain data model boundary for service contracts.

- Introduce a scan-domain `Scan` type in the scan module and use it as the service contract for scan read/create flows.
- Keep `storage::ScanRecord` as a persistence-only type and perform mapping at the scan-service/storage boundary.
- Ensure read endpoints (`get_scan`, `get_scan_status`, `get_scan_result`) return scan-domain types from `ScanService` rather than exposing storage record types.
- Keep storage schema and persistence extensions in `ScanRecord`, but treat `ScanRecord` as storage-internal.
- Implement a scan state coordinator module that composes executor submodules used by service and worker paths.
- Keep the transition executor submodule focused on status persistence + transition telemetry and invoke it via the scan state coordinator.
- Implement a combined execution-state executor submodule for result batch persistence, alert cursor updates, and progress updates.
- Invoke the execution-state executor via the scan state coordinator from worker/service code paths.
- Preserve required ordering semantics in the execution-state executor (for example, alert cursor advancement only after successful result batch persistence).
- Keep scan-domain `Scan` as a pure domain model while infrastructure side effects remain in executor/facade components.

## Phase 4: Stop Flow and Stop-Request Flag

Status: Done (2026-06-11)

Implement strict stop semantics for requested and running scans.

- `requested` + stop:
  - remove from queue
  - transition directly to `stopped`
- `running` + stop:
  - keep status `running` and set `stop_requested=true`
  - signal worker cancellation
  - worker performs graceful stop and transitions to `stopped` while clearing `stop_requested`
- Add configurable stop grace period (default 5 minutes):
  - if exceeded, force stop and transition to `failed`
- If ZAP stop actions fail non-transiently, transition to `failed`.

Phase 4 amendments (2026-06-11):

- Routed `running` scan stop-request persistence through the scan state coordinator execution-state path (`stop_requested=true`) and kept runtime handle responsibilities focused on grace-period enforcement.
- Added configurable stop grace period setting `GREENBONE_WAS_SCAN_STOP_GRACE_PERIOD_SECONDS` with default `300` seconds.
- Added worker-side graceful stop actions for in-flight spider/active scan operations by calling ZAP stop endpoints before finalizing `running` -> `stopped`.
- Added forced-failure handling when stop grace period expires while scan remains `running` with `stop_requested=true`.
- Added failure handling for non-success ZAP stop actions, transitioning scans to `failed`.

## Phase 5: URL Validation and Retry Backoff

Status: Done (2026-06-11)

Implement spec-compliant target validation and resilient external calls.

- Validate comma-separated target URLs:
  - absolute HTTP/HTTPS only
  - trim surrounding whitespace
  - reject user-info
  - reject query strings
  - reject fragments
  - reject whitespace/control characters in URL
  - reject dot patterns as defined by spec
- Return `InvalidUrl` with original value + reason on validation failure.
- Add retry helper with exponential backoff:
  - start delay 1 second
  - configurable max retries (default 10)
  - configurable max delay (default 60 seconds)
- Use retry for transient storage lock/contention and transient ZAP/network failures.
- Exhausted retries transition active scan to `failed`.

Phase 5 amendments (2026-06-11):

- Added scan target URL validation in `src/scan/validation.rs` and integrated it into `create_scan` via `validate_target_urls`.
- Updated validation rules/spec to reject target URLs containing query strings.
- Added retry helper infrastructure in `src/scan/retry.rs` with transient error classification (`IsTransient`) and exponential backoff.
- Added configurable retry settings in `src/config/settings.rs`:
  - `GREENBONE_WAS_SCAN_RETRY_MAX_RETRIES` (default `10`)
  - `GREENBONE_WAS_SCAN_RETRY_MAX_DELAY_SECONDS` (default `60`)
- Wired retry configuration from settings into runtime startup in `src/lib.rs`.
- Implemented retry wrappers for infrastructure-facing components:
  - `RetryingZapClient` for transient ZAP/network failures
  - `RetryingScanStateCoordinator` for transient storage/backend failures
- Removed worker-level retry closure boilerplate in favor of wrapper-based retrying calls.

## Phase 6: Progress Model and Alerts Polling

Status: Done (2026-06-11)

Implement persisted progress stage tracking and per-host calculations, then expose progress via the HTTP `host_info` model.

- Track per target:
  - spider stage state: pending/running/done
  - spider last state from ZAP: running/stopped
  - active scan stage state: pending/running/done
  - active scan percentage 0..100
- Compute per-host progress percentage:
  - `0` if spider is not started
  - `1` if spider is started but not finished
  - `floor(25 + 0.75 * active_scan_percent)` once spider is finished
- Expose progress in HTTP status responses as `host_info` with:
  - counters: `all`, `excluded`, `dead`, `alive`, `queued`, `finished`
  - `scanning`: list of `{ host, progress }` objects for currently scanned hosts
- Poll alerts at configurable interval (default 10 seconds).
- Use pagination start offset from persisted processed-alert count to avoid duplicates.

Phase 6 amendments (2026-06-11):

- Added `Deserialize` to all progress types (`StageState`, `TargetProgress`, `ScanProgress`) so stored JSON progress can be deserialized at read time.
- Fixed `ScanProgress::refresh()` to return `1` for spider-running state instead of `0` (was returning `0` for both pending and running).
- Added `HostInfo` and `HostScanningEntry` to `src/api/dto/scans.rs`; added optional `host_info` field to `ScanStatusResponse` (skipped when `None`).
- Extended `ScanStatusView` in `src/scan/model.rs` with `progress: Option<ScanProgress>`; `Scan::status_view()` now deserializes the stored progress JSON.
- Updated `GET /scans/{id}/status` handler to map `ScanProgress` to `HostInfo` via `progress_to_host_info()` in `src/api/scans.rs`.
- `queued` = targets with pending spider; `finished` = targets with active scan done; `scanning` = targets in between; `excluded`/`dead`/`alive` are `0` (host reachability not yet tracked).
- Alert polling interval was already implemented in Phase 3 (`alert_poll_interval` in worker loop); pagination via persisted `alert_cursor` was already in place.

## Phase 7: Startup Recovery and Wiring

Status: Done (2026-06-11)

Wire scan runtime into service startup.

- In `src/lib.rs` startup path:
  - initialize scan service + queue + worker supervisor
  - run recovery pass: all non-terminal scans become `failed`
  - start worker tasks before serving API traffic
- Extend `AppState` to include scan service handle and resolved scan-runtime configuration values used by handlers/workers.
- Keep API module as transport boundary only (HTTP parsing + response mapping).

Phase 7 amendments (2026-06-11):

- Added `run_startup_recovery` in `src/lib.rs` that calls `list_non_terminal_scans()` and transitions each `requested` or `running` scan directly to `failed` with a `warn!` log; `stored` scans are left untouched.
- Recovery runs after the scan runtime and service are initialized but before `axum::serve` begins accepting connections.
- `AppState` and `ScanService` wiring was already complete from prior phases; no structural changes were required there.

## Phase 8: Observability and Telemetry

Status: Done (2026-06-11)

Complete observability work that depends on retry behavior.

- Log transient ZAP/storage failures as warnings when retries remain.
- Log retry exhaustion as error before transitioning to `failed`.

Phase 8 amendments (2026-06-11):

- Extended `with_retry` in `src/scan/retry.rs` to emit `warn!` on transient failures when retries remain and `error!` when transient retries are exhausted.
- Added operation labels to all retry wrapper call sites in `src/zapclient/mod.rs` and `src/scan/state_coordinator/mod.rs` so retry logs identify the failing operation.
- Added retry observability tests in `src/scan/retry_tests.rs` asserting warning logs for transient retries and error logs for retry exhaustion.

## Phase 9: Tests

Follow repository sidecar test pattern.

- Add unit tests for all valid transitions.
- Add unit tests for invalid transitions (must error).
- Add transition executor submodule tests covering compare-and-swap success, invalid-state/not-found outcomes, and no-telemetry-on-failed-write behavior.
- Add execution-state executor submodule tests covering result-batch + alert-cursor ordering and progress update routing.
- Add scan state coordinator tests covering delegation to the transition executor and execution-state executor.
- Add worker-path tests for:
  - successful completion to `succeeded`
  - requested stop path to `stopped`
  - running stop path via `stop_requested=true` to `stopped`
  - forced stop timeout to `failed`
  - retry exhaustion to `failed`
- Add startup recovery test:
  - non-terminal scans become `failed` on startup.
- Add URL validation tests for all rejection rules.
- Add alert pagination tests ensuring duplicate avoidance.
- Add tests for alert-to-result batch conversion ensuring all converted results are written atomically per alert page.
- Add tests proving alert cursor advancement happens only after successful alert-to-result persistence.
- Add service tests for read commands (`get_scan`, `get_scan_status`, `get_result`) including not-found behavior.
- Add API tests confirming scan endpoints use the service facade and keep storage access out of handler logic.
- Add service/API tests asserting scan creation and deletion emit informational logs.

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
- Confirm status transition telemetry is emitted via the transition executor after successful storage mutation.
- Confirm result persistence, alert cursor updates, and progress updates flow through the execution-state executor via the scan state coordinator.
- Confirm progress exposed by `GET /scans/{id}/status` uses `host_info` with fields `all`, `excluded`, `dead`, `alive`, `queued`, `finished`, and `scanning`.
- Confirm `host_info.scanning` is a list of `{ host, progress }` objects (not strings and not key/value pseudo-maps).
- Confirm per-host progress rules in runtime output:
  - `0` when spider is not started
  - `1` when spider is started but not finished
  - `floor(25 + 0.75 * active_scan_percent)` after spider finished
- Confirm no overall/scan-level percentage field is returned in HTTP status responses.
- Manual API checks:
  - create -> `stored`
  - start (`stored`) -> `requested`
  - worker pick -> `running`
  - stop `requested` -> `stopped`
  - stop running -> `running` + `stop_requested=true` -> `stopped`
  - runtime failure in non-terminal -> `failed`
  - succeeded/stopped/failed/stored deletable

## Non-Goals

- No scan prioritization policy beyond FIFO in this phase.
- No advanced worker resource scheduling (RAM/CPU-based) in this phase.
- No multi-instance distributed queue coordination in this phase.
- No switch to per-scan dedicated ZAP instance in this phase.
- No backward-compatible SQLite schema migration path in this phase.