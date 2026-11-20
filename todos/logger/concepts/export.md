# Optional diagnostic export

**Roadmap coverage:** EXPORT-OPTIONAL. **Source:** [local-only policy](../logger.md#cel-i-zasady), optional export item in the historical roadmap. Requirements: REQ-LOCAL and REQ-SEMANTICS.

## Intended future outcome

If separately authorized, a collector such as Grafana Alloy may read existing structured local files and send them to Loki for Grafana/agent analysis. Grafana is a viewer; Loki is the proposed log store. These are future options, not new dependencies of the Rust tools.

## Current boundary

No export, collector installation, network transmission or backend deployment is authorized. Local logging must remain functional without network access, credentials or Grafana/Loki. Do not add direct delivery to every binary.

Resolve [EXPORT-APPROVAL](../decisions-and-gaps.md#export-authorization) before implementation, including destination, access, local/remote boundary, handling of unmasked operation payloads, retention, replay and collector failure. No masking/export policy is silently invented here.

If approved later, keep high-cardinality run IDs, paths and project identities in JSON/structured metadata, not Loki labels. Choose low-cardinality labels based on the actual deployment contract.

## Future acceptance

Use synthetic data to prove file collection, parsing, queryability, correlation and behavior during collector/network failure; ensure no business operation depends on export. Exact deployment artifacts and validation commands do not exist in this objective and must be established before activating work. Optional export is not part of local logging acceptance.
