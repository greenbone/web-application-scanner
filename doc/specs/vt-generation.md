# VT Generation from ZAP Alert Documentation

## Status

Draft.

## Context

Greenbone WAS needs a NASL VT metadata entry for each ZAP alert that can be
reported by the scanner. The source material is the generated ZAP alert
documentation under `src/feed/tmp/alerts`.

The alert documentation is Markdown with YAML frontmatter. The frontmatter is
the authoritative source for structured fields such as alert ID, status, risk,
CWE, WASC, references, and tags. The Markdown body is descriptive prose and
should be used as supporting text only after the structured fields have been
mapped.

The generated NASL must use canonical NASL metadata primitives instead of
inventing ZAP-specific tags where a canonical field already exists.

## Inputs

The generator reads every Markdown file in `src/feed/tmp/alerts` except
`_index.md`. ZAP release/beta/alpha alerts and PTK `Tool` alerts are handled by
the same generation path and source namespace.

There are two relevant document types:

- `type: alert`: a concrete alert variant. Generate one NASL VT metadata entry
  for each of these files unless the alert is explicitly excluded.
- `type: alertset`: a parent grouping for multiple concrete alerts. Do not
  generate a VT for the parent itself when child `type: alert` files exist.
  Use it as inherited context for the child alerts.

The current corpus contains alertsets such as `10020.md` with child alerts such
as `10020-1.md`. The child alert contains the concrete risk, solution,
references, CWE/WASC mappings, and body text. Parent alertsets mostly contain
the group title, child names, source code URL, alert type, and status.

## Output

The generator produces one NASL file per concrete ZAP alert. The file name
should be deterministic and derived from the source alert ID, for example:

- `zap_10020_1.nasl` for `alertid: 10020-1`
- `zap_40012.nasl` for `alertid: 40012`

Generated files must contain only NASL metadata and the minimal execution stub
needed by the feed until runtime integration is specified. They must not embed
ZAP source code or generated prose that is unrelated to the alert.

Generation is best effort for incomplete ZAP metadata. If an alert has a stable
`alertid`, a usable title/name, and a valid OID, the generator should still
write a NASL metadata file even when optional vulnerability fields such as
`risk`, `solution`, `cwe`, or `wasc` are missing. Missing fields must be
reported through diagnostics, but they must not suppress otherwise valid NASL
output.

## OID Allocation

Each generated VT needs a stable OID derived from the ZAP alert ID.

Greenbone's enterprise base OID is `1.3.6.1.4.1.25623`
(`iso.org.dod.internet.private.enterprise.OpenVAS`). The assigned subtree
`1.3.6.1.4.1.25623.3` is reserved for ZAP alerts.

OID format:

```text
1.3.6.1.4.1.25623.3.<base-alert-id>[.<sub-alert-id>]
```

The generator must map the ZAP `alertid` directly into OID child arcs. It must
not zero-pad, prefix, concatenate, or otherwise encode the base alert ID and
sub-alert ID into a synthetic numeric suffix.

Rules:

- For `alertid: 353-1`, produce `1.3.6.1.4.1.25623.3.353.1`.
- For `alertid: 40012`, produce `1.3.6.1.4.1.25623.3.40012`.
- For `alertid: 10020-4`, produce `1.3.6.1.4.1.25623.3.10020.4`.
- Fail generation if the base alert ID or sub-alert ID is not a non-negative
  decimal integer.

The ZAP `alertindex` field must be recorded for traceability, but it must not
be used directly as the NASL OID suffix because the OID is derived from the
source `alertid` and preserves sub-alerts as separate child arcs.

## Canonical NASL Metadata Mapping

The table below defines the preferred mapping from ZAP documentation fields to
canonical NASL metadata.

| ZAP source field | NASL metadata | Required | Notes |
| --- | --- | --- | --- |
| `alertid` | `script_xref(name:"ZAP-Alert-ID", value:"...")` | Yes | Preserve the exact source ID, including sub-alert suffixes. |
| `alertid` OID arcs | `script_oid(...)` | Yes | Use the OID allocation rules above. |
| `title` | `script_name(...)` | Yes | Use the concrete child title for leaf alerts. If missing, fall back to `name`, then parent alertset name. |
| Generated timestamp | `script_version(...)` | Yes | Use the generation timestamp in NASL version format. |
| `date` | `script_tag(name:"creation_date", value:"...")` | No | Use only if present; otherwise use the generation timestamp. |
| `lastmod` | `script_tag(name:"last_modification", value:"...")` | No | Use only if present; otherwise use the generation timestamp. |
| `alerttype` | `script_category(ACT_GATHER_INFO)` and `script_xref(name:"ZAP-Alert-Type", value:"...")` | Yes | Generated NASL scripts are metadata containers and never execute ZAP directly. |
| Fixed WAS family | `script_family("Web application scanner")` | Yes | Use the existing WAS script family. |
| `risk` | No CVSS metadata by itself | No | ZAP `risk` is coarse source metadata and must not be converted into `cvss_base` or `cvss_base_vector`. See severity mapping below. |
| `alerttype` and source class | `script_tag(name:"qod_type", value:"...")` | Yes | See QoD mapping below. |
| Body text | `script_tag(name:"summary", value:"...")` | Yes | Use the first non-empty normalized paragraph. Use the title as the summary only when no body text exists. |
| `other` | `script_tag(name:"vuldetect", value:"...")` or omit | No | Use only when it describes generic detection behavior. Omit placeholder or instance-specific example values. |
| `solution` | `script_tag(name:"solution", value:"...")` | No | Preserve remediation text, normalized to plain text when present. |
| Derived from `solution` | `script_tag(name:"solution_type", value:"...")` | No | Emit only when it can be derived confidently. See solution type mapping below. |
| `cwe` | `script_xref(name:"CWE", value:"CWE-<id>")` | No | Also deduplicate matching `CWE-*` entries in `alerttags`. |
| `wasc` | `script_xref(name:"WASC", value:"WASC-<id>")` | No | Preserve numeric WASC mappings where available. |
| `references[]` | `script_xref(name:"URL", value:"...")` | No | Emit one URL xref per reference. |
| `help` | `script_xref(name:"URL", value:"...")` | No | Keep the ZAP help URL as a source reference. |
| `code` | `script_xref(name:"URL", value:"...")` | No | Keep the source-code URL as traceability, not as a remediation reference. |
| `alerttags[]` CVE entries | `script_cve_id(...)` | No | Extract tags matching `CVE-YYYY-NNNN...`. |
| Other `alerttags[]` | `script_xref(...)` or omit | No | See taxonomy tag handling below. |
| `techtags[]` | `script_xref(name:"ZAP-Technology", value:"...")` | No | Do not convert these into NASL dependency keys until runtime technology matching exists. |
| `status` | `script_xref(name:"ZAP-Status", value:"...")` | Yes | Preserve `release`, `beta`, `alpha`, or `deprecated`. |
| `alertindex` | `script_xref(name:"ZAP-Alert-Index", value:"...")` | Yes | Traceability only. |
| Parent alertset title | `script_xref(name:"ZAP-Alert-Set", value:"...")` | No | Use when a leaf alert belongs to an alertset. |

Do not create custom NASL `script_tag` names for raw ZAP fields unless the field
cannot be represented by `script_xref`, `script_cve_id`, or an existing
canonical `script_tag` name.

## Category Mapping

Generated NASL scripts are metadata containers. They must never execute ZAP
directly, and their `script_category` must describe the NASL execution behavior,
not the original ZAP rule implementation.

All generated ZAP metadata VTs must use:

```nasl
script_category(ACT_GATHER_INFO);
```

Preserve the original ZAP `alerttype` separately:

```nasl
script_xref(name:"ZAP-Alert-Type", value:"Passive");
```

The source `alerttype` is still useful to consumers that need to distinguish
passive, active, script, WebSocket, client, or tool-originated findings:

| ZAP `alerttype` | NASL category | Rationale |
| --- | --- | --- |
| `Passive` | `ACT_GATHER_INFO` | Metadata for a passive ZAP HTTP response/request observation. |
| `Client Passive` | `ACT_GATHER_INFO` | Metadata for a browser/client observation. |
| `WebSocket Passive` | `ACT_GATHER_INFO` | Metadata for a passive WebSocket observation. |
| `Script Passive` | `ACT_GATHER_INFO` | Metadata for a passive script-based ZAP rule. |
| `Active` | `ACT_GATHER_INFO` | Metadata for an active ZAP rule; the NASL script itself does not probe. |
| `Script Active` | `ACT_GATHER_INFO` | Metadata for an active script-based ZAP rule; the NASL script itself does not probe. |
| `Script Httpsender` | `ACT_GATHER_INFO` | Metadata for a ZAP request/response hook. |
| `Tool` | `ACT_GATHER_INFO` | Metadata for PTK DAST/IAST/SAST modules included in the same generator path as ZAP alerts. |

## Severity Mapping

The ZAP alert docs provide coarse `risk` values, not canonical CVSS scores or
vectors. Do not derive `cvss_base`, `cvss_base_vector`, or `severity_origin`
from `risk`, title, alert type, alert ID, or any other inferred signal.

Generated NASL must emit `cvss_base` and `cvss_base_vector` only when those
values are explicitly present in the source alert documentation. When explicit
CVSS values are absent, omit `cvss_base`, `cvss_base_vector`, and
`severity_origin`.

If ZAP or another authoritative source later provides CVSS scores and vectors
per alert, those source-provided values may be emitted with an appropriate
`severity_origin`.

## QoD Mapping

NASL supports canonical `qod_type` values. Generated VTs must always emit:

```nasl
script_tag(name:"qod_type", value:"remote_analysis");
```

Do not derive stronger `qod_type` values from ZAP runtime evidence during result
import. Do not emit both `qod` and `qod_type`.

## Solution Type Mapping

Set `solution_type` from the best available signal:

- `Mitigation`: default for most generated ZAP alerts because the docs usually
  describe configuration, validation, sanitization, or operational mitigations.
- `VendorFix`: use only when the solution explicitly requires applying a vendor
  patch, upgrading a product, or references a fixed version.
- `Workaround`: use when the solution describes disabling a feature, blocking
  access, or applying a temporary configuration workaround without a vendor fix.
- `WillNotFix`: use for deprecated alerts that are retained only for historical
  compatibility, if those alerts are generated at all.

If no solution text is present, emit `solution_type` only when the generator can
derive it confidently from the alert metadata.

## Taxonomy Tag Handling

`alerttags` contain a mix of standards, policy, and source-system tags. Map
only tags with clear semantics:

- `CVE-*`: emit through `script_cve_id`.
- `CWE-*`: emit through `script_xref(name:"CWE", value:"...")` and deduplicate
  with the numeric `cwe` field.
- `WSTG-*`: emit `script_xref(name:"OWASP-WSTG", value:"...")`.
- `OWASP_2017_*`, `OWASP_2021_*`, `OWASP_2025_*`: emit
  `script_xref(name:"OWASP", value:"...")`.
- `API_2023_*`: emit `script_xref(name:"OWASP-API", value:"...")`.
- `PCI_DSS`, `HIPAA`, and `POLICY_*`: omit from canonical NASL metadata.
- `SYSTEMIC`, `TOOL_PTK`, and other source-classification tags: omit unless a
  consumer needs them for filtering.

The generator must deduplicate xrefs after normalization.

## Text Normalization

The alert files contain YAML strings, Markdown prose, bullets, Unicode bullets,
and escaped JSON-style sequences. Generated NASL metadata should be plain text:

- Parse YAML frontmatter with a YAML parser.
- Strip leading Markdown heading markers from generated tag values.
- Convert Markdown links to `label (URL)` or move the URL to `script_xref`.
- Normalize repeated whitespace while preserving readable sentence boundaries.
- Replace bullet characters with plain ASCII list separators.
- Escape NASL string delimiters and backslashes.
- Do not copy source-code URLs, help URLs, or references into `summary`,
  `insight`, `impact`, or `solution`; represent them as `script_xref`.

## Parent and Child Alert Merging

For a leaf alert with ID `base-sub`:

1. Load the parent alertset `base.md` if it exists.
2. Use the child leaf as the authoritative source for concrete metadata.
3. Fill missing child fields from the parent only for `alerttype`, `status`,
   `code`, `linktext`, and parent group title.
4. Do not inherit parent `title` over a child title.
5. Do not generate a parent VT if concrete child alerts exist.

If an alertset has no concrete child files, generate one metadata-only VT from
the parent alertset. Use the parent title as `script_name`, parent `alertid` for
OID generation, and any available parent fields for traceability. Parent-derived
metadata VTs may omit `risk` and `solution` when the parent alertset does not
define them.

## Deprecated Alerts

Alerts with `status: deprecated` are generated for compatibility with historical
ZAP alert IDs. Generate them with:

- `script_xref(name:"ZAP-Status", value:"deprecated")`
- `solution_type` set to `WillNotFix`
- Summary text that clearly states the alert is deprecated when the source body
  includes a deprecation note

## Example Metadata Skeleton

```nasl
if (description)
{
  script_oid("1.3.6.1.4.1.25623.3.10020.1");
  script_version("2026-06-01T00:00:00+0000");
  script_tag(name:"creation_date", value:"2026-06-01 00:00:00 +0000");
  script_tag(name:"last_modification", value:"2026-06-01 00:00:00 +0000");
  script_name("Missing Anti-clickjacking Header");

  script_category(ACT_GATHER_INFO);
  script_family("Web application scanner");
  script_copyright("Copyright (C) 2026 Greenbone AG");

  script_tag(name:"qod_type", value:"remote_analysis");

  script_xref(name:"ZAP-Alert-ID", value:"10020-1");
  script_xref(name:"ZAP-Alert-Index", value:"1002001");
  script_xref(name:"ZAP-Alert-Type", value:"Passive");
  script_xref(name:"ZAP-Status", value:"release");
  script_xref(name:"ZAP-Alert-Set", value:"Anti-clickjacking Header");
  script_xref(name:"CWE", value:"CWE-1021");
  script_xref(name:"WASC", value:"WASC-15");
  script_xref(name:"OWASP", value:"OWASP_2021_A05");
  script_xref(name:"OWASP-WSTG", value:"WSTG-v42-CLNT-09");
  script_xref(name:"URL", value:"https://www.zaproxy.org/docs/desktop/addons/passive-scan-rules/#id-10020");

  script_tag(name:"summary", value:"The response does not protect against clickjacking attacks.");
  script_tag(name:"solution", value:"Modern web browsers support the Content-Security-Policy and X-Frame-Options HTTP headers. Ensure one of them is set on all web pages returned by the application.");
  script_tag(name:"solution_type", value:"Mitigation");

  exit(0);
}
```

## Validation Requirements

Generation must fail for:

- Missing or duplicate `alertid`.
- Invalid alert ID format.
- Missing `title`/`name`.
- Invalid URL values in `references`, `help`, or `code`.
- OID collisions after direct alert ID arc mapping.

Generation should warn, but not fail, for:

- Missing `cwe` or `wasc`.
- Empty `other`.
- Missing parent alertset for a leaf alert.
- Missing `risk` or `solution` for a generated alert, including leaf alerts and
  parent-derived alertset VTs.
- Unknown `alerttags`.
- Unknown `alerttype`.

Warnings must not prevent writing generated NASL files. The generator may offer
a strict CI mode, such as `--fail-on-warning`, that exits non-zero after writing
or dry-running the candidate set, but default generation must produce all NASL
files that can be rendered without structural errors.

## Open Questions

No open questions remain.
