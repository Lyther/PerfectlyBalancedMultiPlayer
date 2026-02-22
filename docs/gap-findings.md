# SMMS Gap, Risk, and Consistency Findings

> Last updated: 2026-02-22
> Baseline: [`docs/context/concept-zero.md`](context/concept-zero.md)

## 1. Key Design-Bible Features Not Implemented

The items below are either explicitly present in `concept-zero.md` or implied by its goals, but are not implemented in the current codebase.

| Area | Expected in design bible | Current state |
|------|--------------------------|---------------|
| Transport security | Encrypted transport (`SSH`/secure channel) | Plain HTTP only. Optional signing adds authenticity/integrity, not confidentiality. |
| Launcher DB fallback | Read-only fallback from `launcher-v2.sqlite` | Not implemented. Playset extraction uses `dlc_load.json` or local `.mod` directory listing. |
| Relay mode | Optional relay/shared-storage mode for difficult NAT scenarios | Not implemented. Only direct host-to-client pull is implemented. |
| Resume semantics | Robust interrupted-transfer resume (rsync-style `--partial` expectation appears in early sections) | Not implemented as an explicit resumable protocol. Re-run fetch performs diff and re-download. |
| Checksum simulation | Optional prediction of in-game Stellaris checksum | Not implemented. Verification is host-manifest parity only. |
| Full game-tree target (`G`) | Early sections define optional checksum-relevant game file sync | Not implemented (and intentionally out of scope in later appendix direction). |

## 2. Historical Contradictions (Now Explicitly Tracked)

`concept-zero.md` contains two conflicting architectures:

1. Early rsync/SSH and multi-target (`W/L/G`) synchronization
2. Later appendix HTTP pull model without rsync and without base game sync

Current code follows **(2)**. Any document claiming rsync-based transfer or game-file sync is stale.

## 3. Prior Doc Drift / Misleading Claims

These were common inconsistencies before the current doc refresh:

- Claimed rsync core engine, while implementation is HTTP blob pull.
- Claimed launcher SQLite fallback extraction, but code does not read SQLite.
- Claimed parallel hashing via `rayon`, while code hashes in a single-threaded traversal.
- API docs modeled `/manifest` as always unsigned, while host can serve `SignedManifest`.

## 4. Current Technical Risks (As-Built)

These are code-level risks worth tracking even after current hardening:

| Risk | Why it matters | Current mitigation |
|------|----------------|--------------------|
| Live host mutation during sync | Steam can update files after host manifest generation, creating manifest/data drift | Client validates per-file hash before write; mismatches fail fast. |
| Unsigned mode usability tradeoff | If no host key is pinned, authenticity is not guaranteed on untrusted networks | Optional Ed25519 signing + host-key pinning support exists. |
| No confidentiality | Mod content and metadata are sent in plaintext | Operational guidance recommends VPN overlays (Tailscale/ZeroTier). |
| Orphan delete blast radius | Aggressive delete is required for parity but risky if boundaries are wrong | Path validation + managed-root checks + symlink-safe scanning/deletion. |

## 5. Recommended Next Steps (Priority)

1. Add transport confidentiality (TLS or equivalent secure tunnel requirement).
2. Implement `launcher-v2.sqlite` read-only fallback for playset extraction.
3. Add replay resistance/session binding for signed manifest flows.
4. Add explicit interrupted-session resume metadata and cleanup bookkeeping.
