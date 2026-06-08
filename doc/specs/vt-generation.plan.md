# VT Generation Implementation Plan

## Goal

Implement the feed generation feature described in
`doc/specs/vt-generation.md` as an isolated Rust binary. The binary reads ZAP
alert Markdown files, maps their frontmatter and body text to canonical NASL
metadata, and writes deterministic NASL VT metadata files.

The generator must not be part of the HTTP service startup path. It is an
offline build/feed tool that can be run manually, from CI, or from a future feed
packaging job.

## Target Binary

Add a second Cargo binary:

```toml
[[bin]]
name = "greenbone-was-vtgen"
path = "src/bin/greenbone-was-vtgen.rs"
```

The binary owns the CLI and process exit behavior. Reusable generator logic
should live under `src/feed/` so it can be tested without spawning a process.
The existing `greenbone-was` service must not depend on or initialize the feed
generator.

## CLI Contract

Initial command:

```sh
greenbone-was-vtgen \
  --input src/feed/tmp/alerts \
  --output target/generated-feed/nasl \
  --version-date 2026-06-01T00:00:00+0000
```

Required behavior:

- `--input`: directory containing ZAP alert Markdown files.
- `--output`: directory where generated `.nasl` files are written.
- `--version-date`: NASL `script_version` timestamp; default to current UTC
  time.
- `--fail-on-warning`: optional stricter CI mode that returns a non-zero exit
  status when warnings exist after all renderable candidates have been
  processed. It must not suppress default output generation.
- `--dry-run`: optional mode that parses, validates, and reports output paths
  without writing files.

The first implementation can parse CLI arguments manually to avoid adding a
dependency. If the CLI grows, add `clap` in a dedicated change.

## Module Layout

Use this internal structure:

```text
src/bin/greenbone-was-vtgen.rs
src/feed/mod.rs
src/feed/alert_doc.rs
src/feed/generator.rs
src/feed/nasl.rs
src/feed/validation.rs
```

Responsibilities:

- `alert_doc.rs`: parse Markdown files into structured alert documents.
- `generator.rs`: merge parent/child alert metadata and decide which VTs to
  generate.
- `nasl.rs`: encode OIDs, escape NASL strings, normalize text, and render NASL.
- `validation.rs`: collect errors and warnings defined by the spec.
- `greenbone-was-vtgen.rs`: CLI parsing, filesystem traversal, writing files,
  and exit codes.

Keep these modules independent from `api`, `http`, `storage`, and `zapclient`.

## Data Model

Represent parsed alerts with typed structs instead of ad hoc string maps:

- `AlertId`: parsed base ID and optional sub ID.
- `AlertDoc`: frontmatter plus Markdown body.
- `AlertKind`: `Alert` or `AlertSet`.
- `GeneratedVt`: output file name, OID, NASL metadata fields, and rendered NASL.
- `ValidationReport`: errors and warnings with source file paths.

YAML frontmatter should be parsed with `serde_yaml`. Fields that are not needed
for generation can be preserved in a loose map only if diagnostics need them.

## Incomplete Metadata Policy

The generator is a best-effort feed producer. It should generate every NASL file
that can be rendered from structurally valid source data, even when
vulnerability metadata is incomplete.

Implementation rules:

- Treat missing `risk`, `solution`, `cwe`, `wasc`, and empty `other` as
  non-structural metadata gaps.
- Do not invent fallback severity, remediation, CWE, or WASC values.
- Omit `cvss_base`, `cvss_base_vector`, and `severity_origin` when `risk` is
  missing or cannot be mapped.
- Omit `solution` when source solution text is missing or empty.
- Emit `solution_type` only when it can be derived from available metadata, such
  as deprecated status or concrete solution text.
- Keep diagnostics for missing `risk` and `solution` visible as warnings, not
  errors, and do not let them prevent writing the NASL file.

## Implementation Phases

### Phase 1: Parser and Inventory

- Add the `greenbone-was-vtgen` binary entry point.
- Add parser support for Hugo-style frontmatter delimited by `---`.
- Parse all fields used by `doc/specs/vt-generation.md`.
- Skip `_index.md`.
- Classify documents as `alert` or `alertset`.
- Build parent-child relationships from alert IDs, not from file ordering.

Acceptance:

- Unit tests cover leaf alerts, alertsets, deprecated alerts, missing
  frontmatter, malformed YAML, and alert IDs with and without sub IDs.
- A dry parser run over `src/feed/tmp/alerts` reports counts for alert,
  alertset, deprecated, and generated candidates.

### Phase 2: Metadata Mapping

- Implement OID mapping under `1.3.6.1.4.1.25623.3` using direct alert ID
  child arcs.
- Implement deterministic output file names.
- Map canonical NASL metadata exactly as specified.
- Preserve ZAP `alerttype` through `script_xref(name:"ZAP-Alert-Type", ...)`.
- Always emit `script_category(ACT_GATHER_INFO)`.
- Always emit `script_tag(name:"qod_type", value:"remote_analysis")`.
- Generate deprecated alerts with `solution_type` `WillNotFix`.
- Generate parent metadata VTs for alertsets without concrete child files.
- Render metadata tags conditionally when source data is incomplete, following
  the incomplete metadata policy.

Acceptance:

- Unit tests cover OID examples from the spec.
- Unit tests cover parent-child merging and parent-only alertset generation.
- Unit tests cover generated output for alerts missing `risk` and alerts
  missing `solution`; both cases still produce NASL output with the affected
  tags omitted.
- Snapshot-style tests verify representative NASL output for:
  - `10020-1.md`
  - `40012.md`
  - a deprecated alert
  - a parent alertset with no child files

### Phase 3: Text and Tag Normalization

- Normalize Markdown body text into plain NASL tag values.
- Escape NASL string delimiters and backslashes.
- Convert supported `alerttags` to canonical xrefs or `script_cve_id`.
- Map `API_2023_*` and legacy `OWASP_2023_API*` tags to
  `script_xref(name:"OWASP-API", value:"...")`.
- Omit compliance, policy, and source-behavior tags that do not represent
  canonical NASL metadata, including `CUSTOM_PAYLOADS`, `TEST_TIMING`, and
  `OUT_OF_BAND`.
- Deduplicate xrefs after normalization.
- Keep references, help URLs, and source-code URLs as `script_xref(name:"URL",
  ...)`.

Acceptance:

- Tests cover CWE/WASC deduplication, CVE extraction, OWASP/WSTG/API mappings,
  policy/source-behavior tag omission, and URL deduplication.
- Tests cover Unicode bullet normalization and quoted YAML string escaping.

### Phase 4: Validation and Exit Behavior

- Implement spec-defined hard failures:
  - missing or duplicate `alertid`
  - invalid alert ID format
  - missing title/name
  - invalid URLs
  - OID collisions
- Implement spec-defined warnings:
  - missing parent alertset for a leaf alert
  - missing risk or solution for generated alerts, including leaf alerts and
    parent-derived alertset VTs
  - unknown alert tags or alert type
- Suppress normal diagnostics for intentionally optional or omitted fields:
  missing CWE, missing WASC, empty `other`, and omitted source-behavior alert
  tags.
- Return non-zero exit status when errors exist.
- Warnings must not prevent writing NASL files in default generation.
- Return non-zero exit status for warnings only when `--fail-on-warning` is set,
  after writing or dry-running all renderable candidates.

Acceptance:

- CLI tests or integration tests verify exit codes.
- Validation output includes source file path and alert ID where available.
- A corpus run with alerts that only lack optional vulnerability metadata still
  writes NASL files and exits successfully unless `--fail-on-warning` is set.

### Phase 5: End-to-End Generation

- Write generated `.nasl` files to the requested output directory.
- Generate renderable NASL files even when optional vulnerability metadata such
  as risk, solution, CWE, or WASC is missing.
- Do not return early from generation after candidate-level warnings. Only
  structural errors should suppress the specific invalid candidate or fail the
  process.
- Create the output directory if it does not exist.
- Remove only files owned by this generator when replacing output; do not delete
  arbitrary user files.
- Sort output deterministically by alert ID.
- Print a concise summary: input count, generated count, warnings, errors, and
  output directory.

Acceptance:

- Running the binary twice with the same input and `--version-date` produces no
  diff.
- Generated output passes basic NASL syntax smoke checks available in this repo
  or adjacent Greenbone tooling.
- The binary can run without starting the `greenbone-was` HTTP service.

## Testing Strategy

Use focused unit tests for parsing and mapping, plus a small fixture directory
for end-to-end tests. Do not depend on the large untracked
`src/feed/tmp/alerts` corpus for every unit test.

Recommended fixtures:

```text
tests/fixtures/alerts/
  _index.md
  10020.md
  10020-1.md
  40012.md
  deprecated.md
  parent-only.md
```

Keep one optional ignored test or documented manual command for running against
the full ZAP alert corpus.

## Operational Workflow

The feed workflow remains two separate steps:

1. Fetch or refresh alert documentation into `src/feed/tmp/alerts`.
2. Run `greenbone-was-vtgen` to produce NASL metadata output.

The generator should not clone remote repositories. Network fetches belong in
separate tooling such as `src/feed/clone.sh` or CI checkout steps.

## Risks and Mitigations

- YAML/frontmatter variation: use `serde_yaml`, typed optional fields, and clear
  validation messages.
- NASL escaping bugs: centralize string rendering in `nasl.rs` and test quotes,
  backslashes, newlines, and long remediation text.
- Metadata drift: keep `doc/specs/vt-generation.md` as the source of truth and
  add tests for each mapping table.
- Accidental service coupling: expose generator code only through `src/feed` and
  avoid importing it from service modules.
- Generated output churn: require deterministic ordering and an explicit
  `--version-date` for reproducible runs.

## Out of Scope

- Executing ZAP from NASL.
- Downloading or cloning ZAP documentation.
- Packaging generated NASL into a distributable feed archive.
- Serving generated VT metadata through the HTTP API.
- Implementing scan-result import from ZAP alerts.

## Open Questions

No open questions remain.
