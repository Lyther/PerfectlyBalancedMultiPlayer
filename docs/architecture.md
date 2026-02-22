# SMMS Architecture (As Built)

> Status: CURRENT
> Last updated: 2026-02-22
> Design reference: [`docs/context/concept-zero.md`](context/concept-zero.md)

## Design Source of Truth

`concept-zero.md` contains two competing directions:

1. Early rsync/SSH-oriented design
2. Appendix "Review" + "No-Bullshit Edition" HTTP pull model

The implemented code follows **the HTTP pull model** (single binary, host serves manifest + blobs, client pulls and enforces parity).

## Implemented Architecture

### Host (`smms serve`)

- Resolves paths (`game`, `workshop`, `user_data`) via config override or Steam detection
- Extracts active playset from `dlc_load.json` (fallback: `user_data/mod/*.mod`)
- Generates manifest for active playset files only (`BLAKE3`)
- Optionally signs manifest using Ed25519 when `[host].signing_key_path` is configured
- Serves:
  - `GET /manifest` -> `Manifest` or `SignedManifest`
  - `GET /file/*path` -> raw file bytes

### Client (`smms fetch <host>`)

- Downloads manifest
- Verifies signed manifest when host key is configured (`[hosts.<host>].public_key`)
- Validates manifest hashes, version, and load-order format
- Diffs local files by BLAKE3
- Downloads missing/mismatched files; verifies hash before write
- Deletes orphan files under managed roots (symlink-safe checks)
- Rewrites `.mod` descriptor `path=` fields to local paths
- Writes `dlc_load.json`
- Optionally launches `stellaris(.exe)`

### Client (`smms verify <host>`)

- Same manifest retrieval/validation pipeline
- Computes local hashes and reports missing/mismatched files
- No file mutation

## Core Data Model

- `Manifest { version, generated_at, files: BTreeMap<String, String>, load_order }`
- `SignedManifest { manifest, signature }`
- Manifest files map contains `manifest_path -> blake3_hex`
- Load order entries are validated (`mod/ugc_<digits>.mod` or `mod/<name>.mod` with strict charset)

## Platform and Scope

- Target OS: Windows 10/11 and Linux
- macOS: out of scope
- Sync scope:
  - Workshop mods in active playset
  - Local mods in active playset
  - Load order via `dlc_load.json`
- Explicitly not syncing base game trees

## Security Model (Current)

- Integrity/authenticity:
  - Optional Ed25519 signed manifest
  - Per-file BLAKE3 verification before write
- Transport:
  - Plain HTTP (`http://host:port`)
  - No confidentiality, no server identity without key pinning
- Threat model note:
  - Without configured host public key, unsigned manifest mode is allowed

## Known Gaps

See [`docs/gap-findings.md`](gap-findings.md) for:

- key concept-zero features not implemented
- contradictions/legacy claims in previous docs
- current operational risks

## Related Docs

- [Execution Roadmap](roadmap.md)
- [NAT Setup](nat-setup.md)
- [API](api/openapi.yaml)
- [System Overview](diagrams/system-overview.md)
- [Sync Sequence](diagrams/sync-sequence.md)
- [Data Model](diagrams/data-model.md)
