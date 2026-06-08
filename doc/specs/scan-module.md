# Scan module

The `scan` module manages the state of scans by receiving commands from the scanner API endpoint handlers, sending commands or requesting data from the ZAP client and persisting the state in the storage.

The state of a scan consists of the following:
- Scan request data such as preferences or target URLs
- The general activity status, e.g. whether it is new, queued, running or done
- The progress of the scan for each target URL, tracking sub-states such as AJAX spider scan or active scan
- Collected scan results

Started scans are added to queue which will asynchronously start worker threads to perform the actual scan actions.

## Authorization boundary

The scan module is internal and separate from the externally accessible HTTP API.

Authentication is handled by the API server before scan module commands are called.

The scan module assumes requests are already authenticated. Authenticated users have full access to all scans.

API handlers are transport adapters only and must not contain scan lifecycle or persistence logic.

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
- new
- queued
- running
- stop requested
- stopped
- interrupted
- done

Overall and and per-target progress within the `running` status is tracked according to the "Progress model" section below.

All state transitions and progress updates are handled as transactions and persisted. Transactions use shared locks on the scan record to prevent races between concurrent API commands and worker-driven updates.

### State transition matrix

Allowed transitions are defined below. Any transition not listed here is invalid and must return an API error.

| From state | Trigger | To state | Notes |
| --- | --- | --- | --- |
| none | `create_scan` command | `new` | A new scan object is created and persisted. |
| `new` | `start_scan` command | `queued` | Scan is added to the queue. |
| `queued` | worker picked scan | `running` | Worker starts execution. |
| `queued` | `stop_scan` command | `stopped` | Scan worker terminated gracefully or scan is removed from queue before execution. |
| `queued` | worker/internal error | `interrupted` | Error path for non-terminal states. |
| `running` | `stop_scan` command | `stop requested` | Stop has been requested; worker should terminate gracefully. |
| `running` | all targets finished + alerts fetched | `done` | Successful completion path. |
| `running` | worker/internal error | `interrupted` | Error path for non-terminal states. |
| `stop requested` | worker stop completed | `stopped` | Finalized user-requested stop. |
| `stop requested` | worker/internal error while stopping | `interrupted` | Stop flow failed before clean stop completion. |
| `stopped` | none | terminal | No further transitions allowed. |
| `interrupted` | none | terminal | No further transitions allowed. |
| `done` | none | terminal | No further transitions allowed. |

### General error handling

Invalid state transitions will use the error `InvalidTransition` with fields for the original and requested state.

Missing scans will use the error `ScanNotFound`.

Storage errors and ZAP client errors will be forwarded as-is.


### Scan creation

New scans are created with the `create_scan` command, which requires a list of target URLs and a preferences data structure.

A newly created scan instance is assigned the status `new`, a random UUID and the parameters given by the command. It is not added to the queue yet.

The UUID is returned if creation of a scan was successful and used as an identifier for subsequent commands.

### Scan start

Scans with a given id can be started with the `start_scan` command if they have the status `new`. Otherwise the command will result in an error.

The `start_scan` command is not idempotent. Repeated calls after a successful start must return an error.

Starting a scan adds it to the queue and sets it status to `queued`.

### Interrupted scan error handling

If an error occurs within a scan while it is in a state other than `new`, `stopped` or `done`, its status
should be set to `interrupted`, it should be removed from the queue and scan workers should terminate.

For transient errors such as network errors or unavailable locks a configurable retry mechanism should be used instead and the scan should only fail with the `interrupted` when the retry limit is exceeded.

The retry mechanism uses exponential backoff with the delay starting at 1 second. The maximum number of retries is configurable (default: 10 retries) as well as the maximum delay (default: 60 seconds).

On startup, any scans not in the `new` or a terminal state will be set to `interrupted` as it is assumed that the service crashed.

### Running scan

When a scan worker has been launched for a `queued` scan, it sets the scan status to `running`.

If no ZAP context exists for the scan, create a new one named `greenbone-was-{scan_uuid}` and set both context name and context id attributes in the scan. It must ensure URL patterns for the targets are added to the context.

Contexts are currently used to isolate alerts and spider results between scans. Future implementations may instead use separate ZAP instances per scan.

If the scan is interrupted there should be an attempt to stop any spider or active scan and clean up the context created for the scan. The scan status will remain `interrupted`.

Once the context is set up, the worker runs the AJAX spider for each target URL and updates the progress. The AJAX spider timeout is taken from the preferences passed to `create_scan`.

After the spider is finished, the worker runs active scans against the target URLs, updating the progress. The active scan timeout is likewise taken from the `create_scan` preferences.

If either the AJAX spider or active scan times out, a warning is logged and an error result is added to the storage. 

Alert polling and context operations do not have dedicated timeouts; transient failures are handled by the general retry mechanism.

After each poll of the active scan status, the worker also fetches the latest ZAP alerts and adds them to the scan.

When all active scans are finished and all alerts are fetched, the ZAP context is removed and the scan status is set to `done`. Failure to remove the context will not alter the status.

### Scan stop

Scans with a given id can be stopped with the `stop_scan` command if they have the `queued` or `running` status.

The `stop_scan` command is not idempotent. Any call for a scan not currently in `queued` or `running` must return an error.

Stopping a `queued` scan will remove it from the queue and set its status to `stopped`.

Stopping a `running` scan will set its status to `stop requested` and request scan workers to stop. Once the scan worker has stopped, it will set the status to `stopped`.

If the scan worker does not stop after a configurable grace period, it should be shut down forcefully and the scan status set to `interrupted`. The default grace period is 5 minutes.

If a ZAP stop action fails in a non-temporary way, the scan should be interrupted.

If the cleanup of the context fails, the scan status is still set to `stopped`.

### Scan read commands and result fetching

The scan module exposes read commands used by scan API endpoints:
- `get_default_preferences`: returns the available scanner preferences and their default values.
- `get_scan`: returns persisted scan request data and identifiers for a scan id.
- `get_scan_status`: returns lifecycle status and timestamps for a scan id.
- `get_result`: returns a single result by scan id and result index.
- `get_results`: returns a result slice by scan id and optional range.

All scan endpoint data access, including read-only access, is routed through scan module commands rather than calling storage directly from API handlers.

Storage-backed missing data outcomes are mapped to `ScanNotFound` for missing scans and to a result-not-found service error (or forwarded storage error) for missing result indexes.

Transport-only endpoints remain outside the scan command surface:
- `HEAD /scans` metadata endpoint.

Scan results remain available even if the scan is stopped or interrupted. In this case the status also implies that results are partial.

### Deleting Scans

Scans with a given id that are in either the initial `new` state or one of the terminal states `done`, `stopped`, `interrupted` can be deleted with the `delete_scan` .

Trying to delete a scan in any other state will cause an error.

## Queuing mechanism

The queue is FIFO but its design should remain extensible so prioritization mechanisms can be added later.

For now only one worker is allowed to be active but this should be extensible to a configurable number of workers and/or resource limits like minimum free RAM.

If all worker slots are occupied, additional started scans remain in `queued` (backpressure) until a worker slot becomes available.

## Progress model

Progress is represented internally using the per-target variables: 
- A state enum (pending, running, done) for each stage of the scan (spider, active scan)
- The last ZAP state for each each scan stage (`running` or `stopped` for spider, a percentage for active scan).
- An overall percentage per stage is calculated as follows: 25% if the spider is done + 0.75 times the active scan percentage, rounded down.

The per-target percentages are also aggregated into an overall percentage of all targets.

## Alerts polling

Alerts are polled by the scan worker at configurable regular intervals (default: 10 seconds).

Alerts are expected to be served by the ZAP API in a stable order, so to avoid duplicates, pagination is used, starting at the number of already processed alerts.


## Observability

- State transitions must be logged as informational messages and emitted as telemetry events.
- Queue wait time (time between `queued` and `running`) must be emitted as a telemetry event.
- Failed ZAP calls must be logged as warnings when the error is transient and retries remain. Once retries are exhausted, they must be logged as errors.

## Testing

The following must be covered by unit tests using a mock ZAP client and in-memory SQLite storage where needed:

- All valid state transitions.
- All invalid state transition attempts (must return an error).
- Error paths that result in `interrupted` status.
- Startup recovery: non-terminal scans are set to `interrupted` on service restart.

## Notes and open questions

There are currently no open questions.