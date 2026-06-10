# Scan module

The `scan` module manages the state of scans by receiving commands from the scanner API endpoint handlers, sending commands or requesting data from the ZAP client and persisting the state in the storage.

The state of a scan consists of the following:
- Scan request data such as preferences or target URLs
- The general activity status, e.g. whether it is stored, requested, running or succeeded
- The progress of the scan for each target URL, tracking sub-states such as AJAX spider scan or active scan
- Collected scan results

Started scans are added to queue which will asynchronously start worker threads to perform the actual scan actions.

## Authorization boundary

The scan module is internal and separate from the externally accessible HTTP API.

Authentication is handled by the API server before scan module commands are called.

The scan module assumes requests are already authenticated. Authenticated users have full access to all scans.

API handlers are transport adapters only and must not contain scan lifecycle or persistence logic.

## Domain model boundary

The scan module owns a scan-domain `Scan` type used by `ScanService` commands and internal orchestration logic.

The storage layer may use a separate persistence representation (for example `ScanRecord`), but that type is storage-internal and not part of the scan service contract.

Mapping between persistence records and scan-domain values happens at the scan module / storage boundary.

## Scan state coordinator boundary

Scan infrastructure side effects are coordinated through a scan-module scan state coordinator component.

The scan state coordinator composes dedicated infrastructure helpers and provides a single orchestration boundary for:
- status transition persistence + transition telemetry
- execution-state persistence

The scan state coordinator keeps these operations consistent across service and worker code paths while preserving transaction guarantees.

Execution-state persistence is handled by a dedicated execution-state executor submodule owned by the scan state coordinator.

In this spec, execution-state refers to persisted scan progress, persisted alert cursor position, and persisted scan results produced while a scan is running.

The execution-state executor is responsible for:
- result batch persistence
- alert cursor updates with required ordering relative to result batch persistence
- progress updates
- emitting debug logs for progress updates, alert cursor updates, and result persistence operations

## Transition execution boundary

Status transition execution is handled by a transition executor submodule owned by the scan state coordinator.

The transition executor is responsible for:
- applying persisted status updates (including compare-and-swap transitions)
- emitting transition telemetry only when the persistence operation succeeds
- returning typed outcomes for not-found and invalid-state transition attempts

The scan-domain `Scan` type defines transition intent and validity, while the transition executor performs infrastructure side effects.

## Target definition

Targets are comma-separated lists of HTTP or HTTPS URLs with the following rules:
- the URL must be parseable with a standards-compliant URL parser
- only absolute URLs are allowed
- other schemas are rejected
- whitespace before and after the URL is trimmed
- URLs containing whitespace or control characters are rejected
- URLs containing user-info (`username:password@host`) are rejected.
- URLs containing dot patterns are rejected.
- URLs containing fragments (`#...`) are rejected.

If a target URL is rejected, an `InvalidUrl` error containing the given URL and the reason for rejection is raised.

When constructing the ZAP inclusion patterns, any regex metacharacters are escaped.

## Scan states and commands

The main scan states are:
- stored
- requested
- running
- stop requested
- stopped
- failed
- succeeded

Overall and and per-target progress within the `running` status is tracked according to the "Progress model" section below.

All state transitions and progress updates are handled as transactions and persisted. Transactions use shared locks on the scan record to prevent races between concurrent API commands and worker-driven updates.

Transition-triggered status persistence and transition telemetry emission are performed through the transition executor, invoked via the scan state coordinator, to keep behavior consistent across service and worker code paths.

Result batch persistence, alert cursor updates, and progress updates are performed through the execution-state executor, invoked via the scan state coordinator.

### State transition matrix

Allowed transitions are defined below. Any transition not listed here is invalid and must return an API error.

| From state | Trigger | To state | Notes |
| --- | --- | --- | --- |
| none | `create_scan` command | `stored` | A new scan object is created and persisted. |
| `stored` | `start_scan` command | `requested` | Scan is added to the queue. |
| `requested` | worker picked scan | `running` | Worker starts execution. |
| `requested` | `stop_scan` command | `stopped` | Scan worker terminated gracefully or scan is removed from queue before execution. |
| `requested` | worker/internal error | `failed` | Error path for non-terminal states. |
| `running` | `stop_scan` command | `stop requested` | Stop has been requested; worker should terminate gracefully. |
| `running` | all targets finished + alerts fetched | `succeeded` | Successful completion path. |
| `running` | worker/internal error | `failed` | Error path for non-terminal states. |
| `stop requested` | worker stop completed | `stopped` | Finalized user-requested stop. |
| `stop requested` | worker/internal error while stopping | `failed` | Stop flow failed before clean stop completion. |
| `stopped` | none | terminal | No further transitions allowed. |
| `failed` | none | terminal | No further transitions allowed. |
| `succeeded` | none | terminal | No further transitions allowed. |

### General error handling

Invalid state transitions will use the error `InvalidTransition` with fields for the original and requested state.

Missing scans will use the error `ScanNotFound`.

Storage errors and ZAP client errors will be forwarded as-is.


### Scan creation

New scans are created with the `create_scan` command, which requires a list of target URLs and a preferences data structure.

A newly created scan instance is assigned the status `stored`, a random UUID and the parameters given by the command. It is not added to the queue yet.

The UUID is returned if creation of a scan was successful and used as an identifier for subsequent commands.

### Scan start

Scans with a given id can be started with the `start_scan` command if they have the status `stored`. Otherwise the command will result in an error.

The `start_scan` command is not idempotent. Repeated calls after a successful start must return an error.

Starting a scan adds it to the queue and sets it status to `requested`.

### Failed scan error handling

If an error occurs within a scan while it is in a state other than `stored`, `stopped` or `succeeded`, its status
should be set to `failed`, it should be removed from the queue and scan workers should terminate.

For transient errors such as network errors or unavailable locks a configurable retry mechanism should be used instead and the scan should only fail with the `failed` when the retry limit is exceeded.

The retry mechanism uses exponential backoff with the delay starting at 1 second. The maximum number of retries is configurable (default: 10 retries) as well as the maximum delay (default: 60 seconds).

On startup, any scans not in the `stored` or a terminal state will be set to `failed` as it is assumed that the service crashed.

### Running scan

When a scan worker has been launched for a `requested` scan, it sets the scan status to `running`.

If no ZAP context exists for the scan, create a new one named `greenbone-was-{scan_uuid}` and set both context name and context id attributes in the scan. It must ensure URL patterns for the targets are added to the context.

Contexts are currently used to isolate alerts and spider results between scans. Future implementations may instead use separate ZAP instances per scan.

If the scan is failed there should be an attempt to stop any spider or active scan and clean up the context created for the scan. The scan status will remain `failed`.

Once the context is set up, the worker runs the AJAX spider for each target URL and updates the progress. The AJAX spider timeout is taken from the preferences passed to `create_scan`.

After the spider is finished, the worker runs active scans against the target URLs, updating the progress. The active scan timeout is likewise taken from the `create_scan` preferences.

If either the AJAX spider or active scan times out, a warning is logged and an error result is added to the storage. 

Alert polling and context operations do not have dedicated timeouts; transient failures are handled by the general retry mechanism.

After each poll of the active scan status, the worker also fetches the latest ZAP alerts and converts them into scan results in storage.

The alert-to-result conversion uses a dedicated storage function that accepts multiple alerts and persists all corresponding results in one transaction. These writes are coordinated through the execution-state executor, invoked via the scan state coordinator.

The alert cursor must only be advanced after successful commit of the corresponding alert-to-result batch transaction. Cursor advancement is coordinated through the execution-state executor, invoked via the scan state coordinator.

When all active scans are finished and all alerts are fetched, the ZAP context is removed and the scan status is set to `succeeded`. Failure to remove the context will not alter the status.

### ZAP alert to scan result mapping

Each fetched ZAP alert produces exactly one persisted scan result.

The mapping is defined as follows:

| ZAP alert field or condition | Scan result field | Mapping rule |
| --- | --- | --- |
| `risk = Informational` | `type` | Map to `log`. |
| `risk = Low`, `Medium`, `High`, or `Unknown` | `type` | Map to `alarm`. |
| `plugin_id` | `oid` | Copy the ZAP plugin ID verbatim. |
| `name`, `risk`, `url`, `description` | `message` | Format as `<name> (<risk>) at <url>` and append `\n<description>` when the description is non-empty. |
| parsed `url.host()` | `hostname` | Copy the parsed host name when the alert URL is a valid absolute URL. Otherwise store `null`. |
| parsed `url.port_or_known_default()` | `port` | Copy the explicit port, or the scheme default (`80` for HTTP, `443` for HTTPS). If the URL cannot be parsed, store `null`. |
| parsed URL scheme `http` or `https` | `protocol` | Store `tcp`. If the URL cannot be parsed, store `null`. |
| `url` | `ip_address` | Due to limitations of the scanner API, the `ip_address` field is used as a general main location identifier, so insert the URL here. |
| all current ZAP alert fields | `detail` | Store `null` for alert-derived results to remain aligned with the current OpenAPI contract, which reserves `detail` for `host_detail` results. |

This keeps alert conversion deterministic and compatible with the current public result schema while still preserving the essential ZAP finding data in `type`, `oid`, `hostname`, `port`, `protocol`, and `message`.


### Scan stop

Scans with a given id can be stopped with the `stop_scan` command if they have the `requested` or `running` status.

The `stop_scan` command is not idempotent. Any call for a scan not currently in `requested` or `running` must return an error.

Stopping a `requested` scan will remove it from the queue and set its status to `stopped`.

Stopping a `running` scan will set its status to `stop requested` and request scan workers to stop. Once the scan worker has stopped, it will set the status to `stopped`.

If the scan worker does not stop after a configurable grace period, it should be shut down forcefully and the scan status set to `failed`. The default grace period is 5 minutes.

If a ZAP stop action fails in a non-temporary way, the scan should be failed.

If the cleanup of the context fails, the scan status is still set to `stopped`.

### Scan read commands and result fetching

The scan module exposes read commands used by scan API endpoints:
- `get_default_preferences`: returns the available scanner preferences and their default values.
- `get_scan`: returns the scan-domain `Scan` for a scan id.
- `get_scan_status`: returns lifecycle status and timestamps for a scan id.
- `get_result`: returns a single result by scan id and result index.
- `get_results`: returns a result slice by scan id and optional range.

All scan endpoint data access, including read-only access, is routed through scan module commands rather than calling storage directly from API handlers.

Storage record types are not exposed from scan-module service interfaces.

Storage-backed missing data outcomes are mapped to `ScanNotFound` for missing scans and to a result-not-found service error (or forwarded storage error) for missing result indexes.

Transport-only endpoints remain outside the scan command surface:
- `HEAD /scans` metadata endpoint.

Scan results remain available even if the scan is stopped or failed. In this case the status also implies that results are partial.

### Deleting Scans

Scans with a given id that are in either the initial `stored` state or one of the terminal states `succeeded`, `stopped`, `failed` can be deleted with the `delete_scan` .

Trying to delete a scan in any other state will cause an error.

## Queuing mechanism

The queue is FIFO but its design should remain extensible so prioritization mechanisms can be added later.

For now only one worker is allowed to be active but this should be extensible to a configurable number of workers and/or resource limits like minimum free RAM.

If all worker slots are occupied, additional started scans remain in `requested` (backpressure) until a worker slot becomes available.

## Progress model

Progress is represented internally using the per-target variables: 
- A state enum (pending, running, done) for each stage of the scan (spider, active scan)
- The last ZAP state for each each scan stage (`running` or `stopped` for spider, a percentage for active scan).
- An overall percentage per stage is calculated as follows: 25% if the spider is done + 0.75 times the active scan percentage, rounded down.

The per-target percentages are also aggregated into an overall percentage of all targets.

## Alerts polling

Alerts are polled by the scan worker at configurable regular intervals (default: 10 seconds).

Alerts are expected to be served by the ZAP API in a stable order, so to avoid duplicates, pagination is used, starting at the number of already processed alerts.

Fetched alerts are converted and inserted as result records in batches, with one transaction per processed alert page.

For each processed alert page, batch result persistence must succeed before the processed-alert cursor is updated.


## Observability

- State transitions must be logged as informational messages.
- Transition status logs/telemetry are emitted by the transition executor (via the scan state coordinator) after successful persisted transition updates.
- Progress updates, alert cursor updates, and result persistence operations must be logged as debug messages by the execution-state executor.
- Scan creation and scan deletion commands must be logged as informational messages.
- Queue wait time (time between `requested` and `running`) must be emitted as a telemetry event.
- Failed ZAP calls must be logged as warnings when the error is transient and retries remain. Once retries are exhausted, they must be logged as errors.

## Testing

The following must be covered by unit tests using a mock ZAP client and in-memory SQLite storage where needed:

- All valid state transitions.
- All invalid state transition attempts (must return an error).
- Transition executor behavior: telemetry is emitted on successful persisted transitions and suppressed for failed transition writes.
- Execution-state executor behavior: result persistence, alert cursor updates, and progress updates preserve required ordering guarantees.
- Scan state coordinator behavior: transition execution and execution-state persistence are routed through the appropriate underlying executors.
- Error paths that result in `failed` status.
- Startup recovery: non-terminal scans are set to `failed` on service restart.
- Alert-to-result mapping, including `Informational -> log`, all other alert risk levels -> `alarm`, URL-derived host and port extraction, and invalid alert URL fallback behavior.

## Notes and open questions

There are currently no open questions.