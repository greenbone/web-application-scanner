# Alert Index Generation

## Status

Draft.

## Context

The feed generator currently produces NASL metadata files from ZAP alert
documentation. Reviewers and downstream feed tooling also need a
machine-readable index that exposes the alert documentation metadata directly,
without requiring consumers to parse NASL or the original Markdown corpus.

The generated index has a top-level schema version and an `alerts` array
containing normalized alert objects. Unlike the NASL output, the index is a
source metadata inventory. It must include every parsed source alert and
alertset document, except `_index.md`, even when an alertset does not produce
its own NASL VT because child alert VTs exist.

## Output Contract

The feed generator must write `alert-index.json` next to the generated NASL
files in the output directory. The JSON document must validate against
`doc/specs/alert-index.json`.

Top-level structure:

```json
{
  "schema_version": 1,
  "alerts": []
}
```

The generator must emit deterministic JSON:

- Sort alerts by parsed alert ID, with base IDs ordered before their sub-alerts.
- Sort arrays whose order is derived from sets, such as tag categories and
  extracted CVEs.
- Use strings for alert IDs, OIDs, and source scalar values that may be numeric
  in YAML, so JSON consumers do not depend on YAML typing quirks.
- Treat empty strings as absent after trimming whitespace.
- Omit optional normalized properties when the source value is absent or empty.
- Emit empty arrays for required collection properties that have no values, such
  as `cwe`, `cve`, `wasc`, and `references`.
- In `raw_frontmatter`, preserve keys whose source value is an empty string by
  serializing that value as `null`; do not emit empty JSON strings.

## Alert Object Mapping

Each alert object should use snake_case names and preserve these fields when
available:

| JSON field | Source | Notes |
| --- | --- | --- |
| `id` | `alertid` | Required. Serialize as a string and preserve sub-alert suffixes. |
| `oid` | Derived from `alertid` | Use the same OID allocation as NASL generation. |
| `document_type` | `type` | `alert` or `alertset`. |
| `name` | `title`, then `name`, then parent child name fallback | Required. Same title selection policy as NASL generation. |
| `description` | Markdown body | Normalized plain text. Omit when the source body has no descriptive content. |
| `alert_index` | `alertindex` | Preserve as string. |
| `alert_type` | `alerttype` | Preserve source value without canonicalization. |
| `status` | `status` | Preserve source value without canonicalization. |
| `risk` | `risk` | Preserve source value; do not derive CVSS metadata from it. |
| `cvss_base` | `cvssbase` | Emit only when explicitly present. |
| `cvss_base_vector` | `cvssbasevector` | Emit only when explicitly present. |
| `severity_origin` | `severity_origin` | Emit only when explicitly present. |
| `solution` | `solution` | Normalize text. Omit when absent or empty. |
| `other` | `other` | Preserve normalized detection or extra detail text. |
| `cwe` | `cwe` and `alerttags` | Array of `CWE-*` strings, deduplicated. |
| `cve` | `alerttags` | Array of `CVE-*` strings, deduplicated. |
| `wasc` | `wasc` | Array of `WASC-*` strings, deduplicated. |
| `references` | `references` | Array of `{ "type": "url", "value": "..." }` objects. |
| `tech_tags` | `techtags` | Deduplicated string array. |
| `alert_tags` | `alerttags` | Include `raw` plus categorized groups such as `cwe`, `cve`, `compliance`, `owasp`, `policies`, `wstg`, and `misc`. |
| `code` | `code` | Preserve source URL. |
| `help` | `help` | Preserve source URL. |
| `link_text` | `linktext` | Preserve source value. |
| `date` | `date` | Preserve source value. |
| `last_modified` | `lastmod` | Preserve source value. |
| `child_alerts` | alertset `alerts` map | For alertsets, preserve child alert IDs and names. |
| `source.path` | input path | Path to the source Markdown document. |
| `source.alert_id` | raw `alertid` | Original alert ID serialized as a string. |
| `raw_frontmatter` | full YAML frontmatter | Preserve all parsed frontmatter with original source field names. |

`raw_frontmatter` is the compatibility escape hatch for source metadata that is
not yet promoted to a normalized top-level field. It must include all
frontmatter keys from the source document, including unknown keys, nested
alertset child data, and source-specific values that the NASL generator ignores.
Empty strings in raw frontmatter must be serialized as `null` so consumers can
distinguish a present-but-empty source key from an omitted key without treating
`""` as data.

## Tag Categorization

The `alert_tags.raw` array must preserve every source `alerttags` entry. The
generator should then populate categorized arrays using the same classification
rules as NASL metadata generation:

- `cwe`: tags matching `CWE-*`.
- `cve`: tags matching `CVE-*`.
- `owasp`: tags matching `OWASP_2017_*`, `OWASP_2021_*`, `OWASP_2025_*`,
  `API_2023_*`, or legacy `OWASP_2023_API*`.
- `wstg`: tags matching `WSTG-*`.
- `policies`: tags matching `POLICY_*`.
- `compliance`: known compliance tags such as `PCI_DSS` and `HIPAA`.
- `misc`: supported source-classification tags and any otherwise unclassified
  tags.

Unlike NASL generation, the JSON index must not omit policy, compliance, or
source-classification tags. They are source metadata and must remain visible to
index consumers.

## Feed Generator Changes

Add an index-generation path to the existing `src/feed` module:

- Extend `alert_doc.rs` so parsed documents retain the complete YAML
  frontmatter as `serde_yaml::Value` or an equivalent lossless structure that
  can be serialized to JSON.
- Add an `AlertIndexEntry` model that is separate from `GeneratedVt`. The index
  is based on parsed source documents, not the final NASL candidate list.
- Reuse existing alert ID parsing, OID encoding, text normalization, URL
  validation, and tag classification helpers where possible.
- Serialize `alert-index.json` with `serde_json`.
- Keep the HTTP service independent from the feed index generator.

The CLI should keep the existing `--output` contract. A normal generation run
must write both NASL files and `<output>/alert-index.json`. `--dry-run` must
print the index path that would be written without creating the file.

## Validation

The index generation must fail on the same structural source errors as NASL
generation:

- Missing or duplicate `alertid`.
- Invalid alert ID format.
- Missing title/name after fallback resolution.
- Invalid URLs in `references`, `help`, or `code`.
- OID collisions.
- JSON serialization failure.

The index must still include documents that only have optional metadata gaps,
such as missing `risk`, missing `solution`, missing CWE, or missing WASC.
Those gaps should remain warnings when NASL generation already reports them.

## Testing Strategy

Add sidecar tests for the feed module:

- Schema-shape test for a representative alert matching
  `doc/specs/alert-index.json`.
- Parent alertset test proving the index includes the alertset even when NASL
  generation suppresses the parent VT because child alerts exist.
- Raw frontmatter preservation test with an unknown frontmatter key.
- Tag categorization test covering policy, compliance, WSTG, OWASP, CWE, CVE,
  and unclassified tags.
- Determinism test proving two runs over the same fixture produce identical
  JSON bytes.
- CLI dry-run/write tests verifying `alert-index.json` is reported or written
  next to generated NASL files.

## CI and Artifact Policy

Update the feed harness so CI validates and archives the JSON index together
with the generated NASL feed:

- Validate `alert-index.json` against `doc/specs/alert-index.json`.
- Include `alert-index.json` in the generated feed artifact.
- Fail the feed job if NASL generation succeeds but index generation or schema
  validation fails.

The JSON index is an inspection and integration artifact. It is not a signed
release feed package until release packaging policy defines that separately.

## Open Questions

No open questions remain.
