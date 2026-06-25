# General guidance for coding agents

The Greenbone Web Application Scanner (greenbone-was) is a wrapper service for ZAP (Zed Attack Proxy). As such, it functions as both a client for the ZAP API and a server for its own scanner API.

## Service Structure Overview

The code for main the Greenbone Web Application scanner is organized into the following modules:
- api: Route handlers for scanner API server endpoints
- app: General application state and error handling
- config: Configuration settings of the `greenbone-was` service itself.
- http: Scanner API HTTP server and route setup. Does not include route handlers, which are in `api` module.
- logging: Logging and telemetry
- storage: Storage of scan related data like target selection, scan status and results.
- zapclient: Client code for interacting with the ZAP API

## Testing

Unit tests should use the sidecar / companion file pattern, e.g. tests for `xyz.rs` are found in `xyz_tests.rs`.

Testing guidelines for specific modules or features may be found in the `doc/specs` directory.

- Always document the intention of tests. Test names, fixture names, or short
  comments should make clear which behavior, regression, or contract the test
  protects.

### HTTP mock conventions

When using Wiremock (or equivalent HTTP mocking), follow these conventions:

- Prefer endpoint-level helper functions that mount one endpoint behavior at a
  time (for example, one helper per ZAP endpoint path).
- Keep helper abstractions shallow. Avoid deep/builder-style layers that hide
  scenario intent; test setup should remain readable without tracing multiple
  indirections.
- Mount endpoints in the same order they are expected to be requested in the
  scenario flow (for example, context setup -> spider -> active scan -> alerts
  -> passive scan -> cleanup).
- Keep uncommon expectations (special bodies, unusual `expect(n)` counts,
  explicit negative cases) close to the specific test scenario rather than
  hiding them behind generic helpers.
- When introducing shared mock helpers, use them only for obvious repetition;
  prefer local test-file helpers first.
