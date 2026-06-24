# Scan Preferences Implementation Plan

This plan adds scanner preference support to the scan service and aligns the public API contract with `doc/openapi-reference.yml`. The initial preference set contains a `scan_mode` enum preference with values `safe` (disables active scans) and `active` (enables active scans), defaulting to `safe`, plus an `ajax_spider_timeout` preference defined once per scan, with its limit enforced per target.

## Target Files

- `src/scan/mod.rs`
- `src/scan/preferences.rs` (new)
- `src/scan/errors.rs`
- `src/scan/service.rs`
- `src/scan/service_tests.rs`
- `src/scan/progress.rs`
- `src/scan/progress_tests.rs`
- `src/scan/worker.rs`
- `src/scan/worker_tests.rs`
- `src/zapclient/ajaxspider.rs`
- `src/zapclient/ajaxspider_tests.rs`
- `src/zapclient/ascan.rs`
- `src/zapclient/ascan_tests.rs`
- `src/api/dto/scans.rs`
- `src/api/dto/scans_tests.rs`
- `src/api/scans.rs`
- `src/api/scans_tests.rs`
- `src/api/openapi.rs`
- `src/api/openapi_tests.rs`
- `doc/openapi-reference.yml`
- `doc/specs/scan-module.md`

## Phase 1: Preference Registry and Defaults

Status: Planned

Add a scan-owned preference registry that defines the supported scanner preferences.

- Add a new module under `src/scan/` for scanner preferences.
- Store preference metadata and default values in one place.
- Define `scan_mode` as an enum preference with values `safe` and `active`, with default `safe`.
- Define an `ajax_spider_timeout` preference with units in seconds, configured at scan level and applied per target; value `0` means unlimited timeout.
- Keep the preference registry independent from storage and HTTP transport concerns.

## Phase 2: API Contract Alignment

Status: Planned

Update the API DTOs and OpenAPI wiring early so the public contract is stable before service behavior is finalized.

- Ensure `POST /scans` continues to accept preference overrides.
- Ensure `GET /scans/preferences` returns the documented preference list.
- Update OpenAPI annotations and `doc/openapi-reference.yml` so the documented preference list includes `scan_mode` with allowed values `safe|active` and default `safe`, plus `ajax_spider_timeout` described as a scan-level setting enforced per target.

## Phase 3: Scan Service Integration

Status: Planned

Teach the scan service to implement the phase-2 API contract for defaults and scan creation.

- Return the default preference list from `get_default_preferences`.
- Pass `scan_mode` and `ajax_spider_timeout` from scan creation input into the scan service preference resolution path.
- Validate preference input during scan creation.
- Allow unknown preference ids but emit a warning message for each unknown preference.
- Validate `scan_mode` values against the allowed enum values (`safe`, `active`).
- Validate the AJAX spider time limit as a non-negative integer number of seconds, where `0` means unlimited.
- Persist the scan with the effective preference set.
- Keep the persisted scan record shape unchanged.

## Phase 4: Scan Mode Behavior

Status: Planned

Implement mode-driven active-scan behavior using `scan_mode` in scan preferences.

- Resolve the effective `scan_mode` value once per scan from the scan-level preferences passed through the scan service.
- For each target, if `scan_mode=safe`, skip the active-scan stage and proceed to the post-active-scan flow as if that stage completed normally.
- For each target, if `scan_mode=active`, run the active-scan stage as normal.
- Ensure the mode behavior applies consistently for all targets in the scan.
- Emit a debug log indicating that active scan was skipped due to `scan_mode=safe`, including scan id and target.
- Ensure progress/state transitions remain valid in both `safe` and `active` modes.

## Phase 5: Passive Scan Progress Stage

Status: Planned

Add a passive-scan target progress stage that follows active scan (or follows spider directly when `scan_mode=safe`) and contributes to overall percentage.

- Extend target progress state to include a passive-scan stage entered after active-scan completion (or immediately after spider when `scan_mode=safe`).
- Introduce `passive_scan_percentage` in target progress tracking and keep existing stage-state semantics (`pending`, `running`, `done`).
- Implement a temporary placeholder for passive scanning: wait 5 seconds per target, then mark passive-scan stage as done.
- Treat correct passive-scan progress integration beyond the placeholder wait as out of scope for this plan.
- Update overall percentage calculation so the existing spider-crawl contribution remains included:
	- old post-spider formula: `25 + 0.75 * active_scan_percentage`
	- new post-spider formula: `25 + 0.7 * active_scan_percentage + 0.05 * passive_scan_percentage`
- Keep other parts of progress calculation unchanged.

## Phase 6: AJAX Spider Limit Enforcement

Status: Planned

Implement runtime enforcement of the scan-level AJAX spider limit with per-target timers.

- Resolve `ajax_spider_timeout` once for a scan before target iteration begins.
- Start a fresh timer for each target when its AJAX spider stage starts.
- Pass the configured limit to ZAP AJAX spider start APIs if supported by the request contract.
- Enforce timeout locally in worker control flow so per-target limits are guaranteed even if ZAP-side timeout behavior changes.
- Treat `ajax_spider_timeout=0` as unlimited and skip local timeout enforcement while still running normal spider stage polling.
- When a target exceeds its limit:
	- stop the target's AJAX spider activity,
	- emit an info log with scan id, target, configured limit, and elapsed seconds,
	- continue the remaining scan steps for the same target as if the spider stage finished normally,
	- continue processing remaining targets unless a stop request or terminal worker error occurs.
- Keep timeout accounting independent per target (no shared global budget across all targets).
- Ensure the default behavior (when preference is omitted) is deterministic and documented.

## Phase 7: Tests and Documentation

Status: Planned

Add focused coverage for the new preference behavior and update any stale scan-module docs.

- Cover preference serialization and deserialization.
- Cover scan-service default preference behavior.
- Cover warning behavior for unknown preference ids.
- Cover scan-service handling of scan-level `ajax_spider_timeout` values.
- Cover scan-service propagation and resolution of `scan_mode` for runtime behavior.
- Cover worker behavior for `scan_mode=safe`, including active-scan stage skip, debug logging, and valid per-target flow completion.
- Cover worker behavior for `scan_mode=active`, including normal active-scan execution.
- Cover worker enforcement behavior for per-target AJAX spider limits, including timeout, info logging, continuation of same-target follow-up stages, and continuation to next target.
- Cover progress-model behavior for passive-scan stage transitions and placeholder 5-second completion.
- Cover updated percentage formula using `25 + 0.7 * active_scan_percentage + 0.05 * passive_scan_percentage`.
- Cover API handler delegation and OpenAPI schema coverage.
- Update `doc/specs/scan-module.md` if it still describes the placeholder preference behavior.

## Verification

- Run `cargo fmt --all -- --check`.
- Run targeted unit tests for `src/api/dto/scans.rs`, `src/api/scans.rs`, `src/api/openapi.rs`, and `src/scan/service.rs`.
- Run targeted worker and ZAP client tests for `scan_mode=safe` skip behavior and `scan_mode=active` active-scan behavior.
- Run targeted worker and ZAP client tests for AJAX timeout enforcement and timeout signaling.
- Run targeted progress tests for passive-scan stage behavior and revised percentage math.
- Run the broader scan and API test suites after the focused tests pass.
- Inspect the generated OpenAPI output to confirm `/scans/preferences` returns the documented array shape and that `scan_mode` (with values `safe|active`) plus `ajax_spider_timeout` appear with correct defaults and descriptions.

## Decisions

- Keep the current persisted scan record shape unchanged.
- Treat the preference registry as the source of truth for defaults and documented metadata.
- Allow unknown preference ids and emit warning messages instead of rejecting scan creation.
- Pass `scan_mode` through scan creation to the scan service.
- In `scan_mode=safe`, skip active-scan stage; in `scan_mode=active`, run active scan.
- Treat the AJAX spider time limit as a scan-level seconds value and enforce it independently for each target in the scan.
- Treat `ajax_spider_timeout=0` as unlimited.
- Introduce passive-scan progress as a temporary 5-second per-target placeholder stage after active scan (or after spider when `scan_mode=safe`). Full handling of the passive scan is out of scope for this plan.

## Noted Deviations

- Deviation from Phase 6 local enforcement: the worker no longer stops AJAX spider scans locally when timeout is exceeded. Instead, it sets the ZAP AJAX spider option (`ajaxSpider/setOptionMaxDuration`) before each spider run and relies on ZAP-side timeout behavior.
- Deviation from original default timeout: the default `ajax_spider_timeout` is `3600` seconds (60 minutes) instead of `0`.
