# Concept map and source coverage

This catalog owns topic/specification boundaries, not task status. The complete 216-line original input was read during migration. [logger.md](logger.md) is preserved intact; links below are archival provenance, not the routine reading route.

## Concept ownership

| Concept | Responsibility | Primary specification | Relationships |
| --- | --- | --- | --- |
| Foundation/storage | Sessions, location, settings, payload files, delivery health, retention | [Foundation](concepts/foundation.md) | Provides infrastructure to all local producers. |
| Process diagnostics | Shared runner attempts, outcomes, arguments/environment and stream observation | [Processes](concepts/processes.md) | Uses storage; feeds profile/restore diagnostics; has an unresolved stream boundary. |
| Profile operations | Manager preparation, state changes, activation, validation and maintenance | [Profiles](concepts/profiles.md) | Uses process diagnostics where native CLIs are invoked. |
| Restore diagnostics | WordPress plan/apply, backup, guards, import, localization and recovery | [Restore](concepts/restore.md) | Uses runner and storage; does not change database safety rules. |
| MCP preparation | Configuration, dotenv, environment and executable selection before handoff | [MCP](concepts/mcp.md) | Leads to wrapper or exec paths; is not bridge HTTP observation. |
| Novamira | Rust guard observations and bridge/container HTTP diagnostics | [Novamira](concepts/novamira.md) | Uses MCP context, payloads, and a channel decision. |
| Handoff observation | Continuity after exec, signals, statuses and cleanup | [Handoff](concepts/handoff.md) | Uses storage/correlation; requires a selected observation contract. |
| Future export | Explicitly optional collector/backend integration | [Export](concepts/export.md) | Local operation must remain independent; approval unresolved. |
| Cross-cutting quality | Data fidelity, semantic preservation, compatibility, independent oracles and VM checks | [Quality](quality-and-acceptance.md) | Governs every implementation task. |

## Complete source-section disposition

| Original section | Classification | Working owner/destination |
| --- | --- | --- |
| Title and “Cel i zasady” | Goal, scope and requirements | [Working requirements](knowledge-base.md#requirements). |
| “Obecny kontekst i architektura” | Context, code observations, illustrative diagram/dependencies | Concept specifications and [decisions](decisions-and-gaps.md); old source line numbers are not live evidence. |
| “Kontrakt diagnostyczny” | Requirements plus one illustrative JSON record | [Working requirements](knowledge-base.md#requirements), [quality](quality-and-acceptance.md), producer specs. Example field values are not mandatory fixtures. |
| “Novamira i granice obserwowalności” | Required coverage and an unresolved integration | [Novamira](concepts/novamira.md) and NOVAMIRA-CHANNEL. |
| “Lokalizacja, retencja i kompletność” and its binding-decision subsection | Accepted infrastructure contracts, including supplements and exceptions | [Working foundation contracts](knowledge-base.md#accepted-foundation-contracts), [foundation spec](concepts/foundation.md). |
| “Procesy i przekazanie sterowania” | Target requirement and unselected technical alternatives | [Handoff](concepts/handoff.md), HANDOFF-CONTRACT. Async-writer alternatives do not override the accepted synchronous foundation. |
| “Etapy realizacji” five-item list | Five delivery stages; historical routing replaced by explicit active stage sections | [Active roadmap](roadmap.md), with stages 1–5 as main headings and tasks beneath them. |
| “Ograniczony fundament JSONL” | Historical increment and failures/superseded incompleteness | Preserved archive; latest foundation evidence is linked from the active roadmap. |
| “Przywrócenie i weryfikacja Rust 1.85” | Historical compatibility remediation and evidence | Preserved archive; current baseline and future compatibility rule in [quality](quality-and-acceptance.md). |
| “Podłączenie JSONL wyłącznie do menedżera” | Historical increment, isolation issues and evidence | Preserved archive; infrastructure owner is [foundation](concepts/foundation.md). |
| “Kontekst wywołania menedżera” | Historical increment, interrupted/resumed verification and byte format | Preserved archive; accepted byte fidelity is retained in [working knowledge](knowledge-base.md). |
| “Kryteria akceptacji przyszłej implementacji” | Whole-objective quality/coverage requirements | [Quality](quality-and-acceptance.md), [coverage matrix](coverage-matrix.md). |
| “Decyzje wymagające doprecyzowania” | Unknowns and approval boundaries | [Decisions and gaps](decisions-and-gaps.md). |
| “Domknięcie etapu 1”, “Wykonany zakres”, “Faktyczna weryfikacja i artefakty” | Current accepted infrastructure scope and recorded evidence | [Stage 1](roadmap.md#etap-1-fundament-loggera), [foundation spec](concepts/foundation.md). |
| “Granice gotowego fundamentu” | Limitations and deferred responsibilities | Foundation limits plus producer/handoff/export specs; no claim that payload storage already captures process/HTTP data. |

Historical “not complete” claims remain historical; they do not compete with the later foundation acceptance. The original claim that all future stages were unstarted is imported only as the migration baseline, not maintained in the archive.
