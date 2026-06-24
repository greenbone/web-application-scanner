# HTTP Mock Refactor Plan

Status: In Progress (2026-06-24)

## Goal

Improve readability and maintainability of tests that use HTTP mocks by extracting repeated Wiremock setup into focused helper functions while preserving test intent and behavior.

## Decisions

- Keep this plan implementation-agnostic.
- Prioritize readability over reducing line count.
- Keep abstraction depth low.
- If introducing an additional abstraction layer, pause and confirm before proceeding.
- Keep helpers local by default, but allow shared helpers when duplication is obvious.
- Use `.expect()` helper defaults only for common cases.
- Use a hybrid `.expect()` approach: keep common-case defaults in helpers and keep uncommon overrides inline.
- Run full test suite validation.
- For `src/scan/worker_tests.rs`, use endpoint helpers only in the first pass.
- Re-evaluate scenario struct introduction only after Phase 1 if readability is still insufficient.

## In-Scope Test Files

- src/scan/worker_tests.rs
- src/zapclient/alert_tests.rs
- src/zapclient/ajaxspider_tests.rs
- src/zapclient/ascan_tests.rs
- src/zapclient/context_tests.rs

Estimated in-scope tests:

- src/scan/worker_tests.rs: 13
- src/zapclient/alert_tests.rs: 5
- src/zapclient/ajaxspider_tests.rs: 13
- src/zapclient/ascan_tests.rs: 9
- src/zapclient/context_tests.rs: 16

Total: 56

## Non-Goals

- No production-code changes.
- No behavior changes in test assertions.
- No framework migration away from wiremock.

## Refactor Strategy

### 1. Endpoint-level helpers

Extract small helpers that mount a single endpoint and response behavior.

Examples:

- mount_context_new_ok(server)
- mount_context_include_ok(server)
- mount_context_remove(server, status_code)
- mount_ajax_spider_set_max_duration(server, integer, expected_calls)
- mount_ajax_spider_scan_ok(server)
- mount_ajax_spider_status(server, status)
- mount_ajax_spider_stop(server, status_code, expected_calls)
- mount_ascan_scan(server, body, expected_calls)
- mount_ascan_status(server, status_code, body, expected_calls)
- mount_ascan_stop(server, status_code, expected_calls)
- mount_alerts_page(server, start, body)

### 2. Scenario bundles

Compose the endpoint-level helpers into scenario helpers where repeated sequences exist.

Examples:

- mount_context_flow_ok(server)
- mount_context_cleanup(server, status_code)
- mount_safe_mode_without_active_scan(server)
- mount_running_active_stage_stoppable(server)

### 3. Keep test intent explicit

- Keep special-case body matchers and expect counts close to test scenarios.
- Avoid a single giant builder that hides behavior.
- Keep abstraction depth low and add layers only when clearly justified.
- Keep uncommon `.expect()` overrides inline so exceptional behavior remains visible.

## Phased Rollout

### Phase 1: src/scan/worker_tests.rs

Status: Done (2026-06-24)

- Extract repeated context lifecycle mounts.
- Expand existing helper style already used in this file.
- Replace repetitive blocks in mock_zap_server_* functions with helper calls.
- Do not introduce a scenario struct in this phase.

Acceptance:

- Same endpoint paths, bodies, and expect(n) semantics.
- Existing test names/assertions unchanged.
- Improved readability with shorter scenario setup.

Validation recorded:

- `src/scan/worker_tests.rs` compiles without errors after refactor.
- Worker runtime tests executed and passed.
- Full test suite executed and passed.

### Phase 2: src/zapclient/context_tests.rs

- Extract helpers for context action success/failure patterns.
- Keep edge-case response payloads visible per test.

Acceptance:

- Repeated mount boilerplate reduced.
- Error handling test clarity preserved.

### Phase 3: src/zapclient/ajaxspider_tests.rs and src/zapclient/ascan_tests.rs

- Introduce endpoint-focused helpers for scan/status/stop actions.
- Keep status parsing and unexpected content tests explicit.

Acceptance:

- Less repeated setup.
- Parsing and validation behavior remains obvious in tests.

### Phase 4: src/zapclient/alert_tests.rs

- Add helpers for paginated alerts and malformed payload responses.
- Preserve visibility of paging parameters and response body structure.

Acceptance:

- Reduced duplication with clear pagination intent.

## Helper Placement

Default:

- Keep helpers local to each test file first.
- Allow promotion to shared test support module when duplication is obvious.

## Validation

After each phase, run targeted tests and then the full test suite.

- cargo test scan::worker_tests -- --nocapture
- cargo test zapclient::context_tests -- --nocapture
- cargo test zapclient::ajaxspider_tests -- --nocapture
- cargo test zapclient::ascan_tests -- --nocapture
- cargo test zapclient::alert_tests -- --nocapture
- cargo test -- --nocapture

## Risks and Mitigations

- Risk: over-abstraction hides endpoint behavior.
  - Mitigation: keep helpers narrow and endpoint-specific.

- Risk: altered expect(n) counts change semantics.
  - Mitigation: preserve and review each expect(n) during refactor.

- Risk: debugging gets harder with indirection.
  - Mitigation: start with local helpers and clear names.

## Done Definition

- All in-scope test files use helper-based HTTP mock setup where repetition exists.
- No behavior changes in tests.
- Readability improves without losing scenario-specific clarity.
- All affected tests pass.
