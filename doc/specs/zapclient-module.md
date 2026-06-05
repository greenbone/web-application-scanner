# ZAP Client module

The Rust module `zapclient` is used to access a subset of the ZAP API required by the Greenbone Web Application Scanner.

## API specification

The full ZAP OpenAPI spec can be found at [https://raw.githubusercontent.com/zaproxy/zap-api-docs/refs/heads/main/openapi.yaml](https://raw.githubusercontent.com/zaproxy/zap-api-docs/refs/heads/main/openapi.yaml).

The `zapclient` module does not offer functions for the full API and is limited to the JSON endpoints for the following:
- ajaxspider (Crawling the target website with the Ajax Spider)
- alerts (Retrieving ZAP alerts, i.e. found vulnerabilities)
- ascan (Active vulnerability scans)
- context (Managing website / scan contexts)

## Module structure

The client object type is split into general functionality like initialization and error types and separate files for groups of endpoints with a common base path, e.g. the `alert` module handles JSON endpoints starting with `JSON/alert`.

## HTTP request and response handling

### General rules

All requests require API key authentication unless explicitly noted otherwise.

To avoid including the API key in request URLs, prefer to use HTTP methods other than GET (usually POST) if available.

If a ZAP fetch endpoint supports pagination parameters, the client should also offer these.

Id parameters like `contextId` are handled as strings even if they usually contain numeric values.

Contexts can be identified by `contextName` or `contextId` depending on the endpoint as documented in the ZAP OpenAPI spec.

Validation of string values should be case-sensitive unless explicitly noted otherwise.

### Error handling

Client functions should handle these errors that can occur during HTTP requests:

- An `UnexpectedStatus` error is raised if the server returns an unexpected HTTP status.
- A `ParseResponse` error is raised if the server returns a valid HTTP status but the response body cannot be deserialized.
- An `UnexpectedContent` error is raised if the server returns a valid HTTP status, the response body can be parsed but it contains otherwise unexpected data, e.g. numbers are out of range.

### Rules for specific endpoints

AJAX Spider status values are normalized case-sensitively to "stopped" and "running".

Any other AJAX Spider status value must raise an `UnexpectedContent` error.

The active scan status is expected to be a percentage value (valid range `0..=100`).

Active scan status values outside `0..=100` must raise an `UnexpectedContent` error.

Context action endpoints (`removeContext`, `includeInContext`) require exact `"OK"` in `Result`.

## Tests

Tests must follow the general test guidelines for this repository (see `AGENTS.md`).

Tests for API endpoints use a mock HTTP server like `wiremock` to verify the following:

- If requests are valid and the server returns the expected response, the client object gives the expected result value.

- All relevant request errors outlined in ´Error handling` are handled.


## Open questions and notes

- ToDo: Add pagination support (`start`, `count`) to `get_alerts`.