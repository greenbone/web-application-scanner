# Passive Scan Refactor Plan

Status: Done (2026-06-25)

## Goal

Replace the passive-scan placeholder wait with a ZAP-driven workflow based on
the records-to-scan endpoint.

- When passive scan starts, fetch the initial remaining record count from
  JSON/pscan/view/recordsToScan.
- Poll the same endpoint regularly and keep the current remaining count in
  worker state.
- Recalculate passive progress whenever the remaining count decreases.
- Mark passive scan done when the remaining count reaches 0.

## Decisions

- Keep this plan implementation-agnostic.
- Reuse existing worker poll cadence (scan_poll_interval); do not add a new
  runtime setting.
- Keep passive progress monotonic by tracking the lowest observed
  records-to-scan value.
- Treat endpoint payload contract as fixed: JSON object with field
  "recordsToScan" containing a string-encoded non-negative integer.
- Treat recordsToScan as per-target progress input under current runtime
  assumptions, even though the endpoint reflects global ZAP state:
  - concurrent workers are expected to run on isolated ZAP instances once
    multi-worker concurrency is supported
  - targets are processed sequentially within a single worker/scan
- Read initial_records immediately when entering passive scan phase (before
  waiting for the first poll interval tick).
- Use immediate running-then-done semantics when initial_records is zero:
  enter passive running state, read initial_records, then transition directly
  to passive done if the value is 0.
- If recordsToScan increases between polls, keep progress monotonic (ignore
  the increase for percentage calculation) and emit a debug log with the
  previous and current values.
- Keep logging minimal and consistent with existing runtime observability:
  - temporary recordsToScan increases: debug log (already specified)
  - repeated endpoint unavailability: rely on standard retry logs
    (warn on retries, error on retry exhaustion)
  - invalid endpoint content: rely on existing worker failure-path error logs
    when scan execution fails
- Use the standard retry mechanism for recordsToScan endpoint calls; if retry
  attempts are exhausted, fail the scan via existing worker failure handling.
- Use single-zero completion in the current implementation: when
  current_records == 0, complete passive phase without an additional
  confirmation read.
- Keep consecutive-zero confirmation as a future hardening option if runtime
  evidence shows zero-value jitter.
- Use stop-first race handling in the passive loop:
  - evaluate stop_requested before completion checks
  - if stop is requested, stop path wins immediately
  - evaluate completion (current_records == 0) only when no stop is pending
- Persist both raw passive-scan counts and the derived percentage:
  - raw counts for observability/diagnostics (initial_records and current_records)
  - percentage/state as canonical progress consumed by existing API/status logic
- Use a deterministic passive progress formula with explicit floor rounding and
  clamping.
- Preserve passive-stage stop behavior (no separate ZAP passive-stop action).
- Remove obsolete passive placeholder-duration runtime configuration.

## Passive Progress Formula

Definitions:

- initial_records: records-to-scan value read once at passive phase start
- current_records: most recent records-to-scan value from polling
- min_records_seen: minimum observed current_records value during the phase

Computation:

- If initial_records == 0, passive progress is 100 and passive state is done.
- Otherwise, use current_effective = min(current_records, min_records_seen).
- Compute ratio = (initial_records - current_effective) / initial_records.
- Compute percentage_raw = floor(100 * ratio).
- Clamp passive_progress_percentage = clamp(percentage_raw, 0, 100).

Update rules:

- Recalculate only when current_records decreases below min_records_seen.
- Persist raw count updates (initial/current) and persist percentage/state when
  passive_progress_percentage changes.
- When current_effective reaches 0, mark passive state done (percentage 100).

## Target Files

- src/zapclient/mod.rs
- src/zapclient/pscan.rs (new)
- src/zapclient/pscan_tests.rs (new)
- src/scan/worker.rs
- src/scan/worker_tests.rs
- src/lib.rs

## Non-Goals

- No API contract changes for scan status endpoints.
- No changes to passive-scan weighting in progress calculations.
- No migration of unrelated worker polling or retry behavior.

## Open Questions

- No unresolved questions currently.

## Phased Rollout

### Phase 1: Add ZAP recordsToScan client support

Status: Done (2026-06-25)

- Add a new pscan module and wire it through zapclient exports.
- Implement a ZapClient method that calls JSON/pscan/view/recordsToScan,
  sends apikey, parses numeric content, and validates non-negative values.
- Add RetryingZapClient wrapper support for this operation.
- Add sidecar tests for success, HTTP errors, parse errors, and invalid
  content values.

Acceptance:

- Endpoint wrapper follows existing zapclient error-handling conventions.
- Retry wrapper parity matches other zapclient operations.
- New pscan tests pass.

Validation recorded:

- pscan tests executed and passed.
- zapclient suite executed and passed.

### Phase 2: Replace passive placeholder workflow in worker

Status: Done (2026-06-25)

- Replace timer-based passive phase logic with records-based polling.
- Fetch initial records-to-scan count at passive phase start.
- Persist initial/current records-to-scan values in progress payload.
- If initial count is zero, finish passive stage immediately.
- Poll records-to-scan on scan_poll_interval until count reaches zero.
- Recompute passive percentage when the count decreases versus previously
  observed minimum.
- Apply the agreed passive progress formula with floor rounding and
  0..100 clamping.
- Persist raw count updates and persist percentage/state when effective
  passive percentage changes.
- Keep existing stop-request handling path for passive stage.

Acceptance:

- Passive phase completion is driven by records-to-scan reaching zero.
- Progress updates occur only on decreasing counts.
- Raw records counts are persisted for observability while percentage/state
  remains the canonical progress signal.
- Stop handling behavior in passive phase is unchanged.

Validation recorded:

- progress tests executed and passed.
- worker runtime tests executed and passed.

### Phase 3: Remove placeholder runtime config

Status: Done (2026-06-25)

- Remove DEFAULT_PASSIVE_SCAN_PLACEHOLDER_DURATION and related config fields
  from scan runtime configuration.
- Remove remaining placeholder-duration references in worker logic and startup
  config wiring.
- Update affected test config initializers.

Acceptance:

- No runtime code references passive_scan_placeholder_duration.
- Runtime and tests compile with updated config structs.

Validation recorded:

- worker runtime tests executed and passed.

### Phase 4: Update worker runtime tests

Status: Done (2026-06-25)

- Add records-to-scan mock helpers in worker tests.
- Update passive-stage stop test setup to use records-driven passive flow.
- Add regression coverage for:
  - progress updates only when records decrease
  - passive phase completion when records reach zero

Acceptance:

- Worker tests validate records-driven passive lifecycle behavior.
- Existing stop semantics remain verified.

Validation recorded:

- worker runtime tests executed and passed.

### Phase 5: Add pscan sequence-sidecar regressions

Status: Done (2026-06-25)

- Add sequence-oriented sidecar tests in `src/zapclient/pscan_tests.rs` for
  mixed response progressions (for example, valid schema transitions and
  malformed content boundaries across repeated calls).
- Keep helper abstraction local and shallow in the pscan test file.
- Preserve explicit endpoint contract assertions for
  `JSON/pscan/view/recordsToScan`.

Acceptance:

- `pscan_tests` include targeted sequence regressions that remain easy to read.
- Existing pscan contract/error tests remain intact.

Validation recorded:

- cargo test pscan_tests -- --nocapture passed.
- cargo test zapclient -- --nocapture passed.

## Validation

After implementation, run targeted tests and then broader zapclient coverage.

- cargo test pscan_tests -- --nocapture
- cargo test worker_tests -- --nocapture
- cargo test progress_tests -- --nocapture
- cargo test zapclient -- --nocapture

Additional validation for Phase 5:

- cargo test pscan_tests -- --nocapture

Closing validation recorded (2026-06-25):

- cargo test pscan_tests -- --nocapture passed.
- cargo test worker_tests -- --nocapture passed.
- cargo test progress_tests -- --nocapture passed.
- cargo test zapclient -- --nocapture passed.

## Risks and Mitigations

- Risk: records-to-scan value can increase transiently, causing backward progress.
  - Mitigation: update progress only when current count drops below prior minimum.

- Risk: excessive progress persistence writes during polling.
  - Mitigation: persist percentage/state only on effective percentage change;
    raw counts are persisted as observability data.

- Risk: endpoint payload shape differs from assumptions.
  - Mitigation: explicit parse and content-validation tests in pscan sidecar tests.

## Done Definition

- Passive scan no longer uses placeholder timing.
- Worker uses records-to-scan polling for passive-stage progress and completion.
- Placeholder runtime config is removed.
- New and updated tests pass with unchanged stop semantics.
