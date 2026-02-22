# SMMS Execution Roadmap (Reality-Based)

> Last updated: 2026-02-22

## Phase 0: Foundation

- [x] HTTP manifest + blob transfer prototype (`axum` + `reqwest`)
- [x] Path auto-detection from Steam libraries
- [x] Basic playset extraction from `dlc_load.json`

## Phase 1: Functional MVP

- [x] `smms init`, `smms serve`, `smms fetch <host>`, `smms verify <host>`
- [x] Active-playset-only manifest generation
- [x] BLAKE3 diff engine (missing / mismatched / orphan delete)
- [x] Descriptor rewrite (`path=` in `.mod`)
- [x] Load order apply via `dlc_load.json`
- [x] Optional launcher bypass (`fetch` launches game unless `--no-launch`)

## Phase 2: Safety Hardening

- [x] Optional backup before overwrite (`--backup`)
- [x] Manifest validation (version/hash/load-order shape)
- [x] Path traversal defenses in backend resolution
- [x] Symlink-safe hashing and orphan scanning
- [x] Atomic file writes with temp + rename
- [x] Delete-time safety checks for orphan cleanup

## Phase 3: Authenticity

- [x] Ed25519 manifest signing (`smms gen-keypair`)
- [x] Signed-manifest verification on client when host key configured
- [x] Fail-closed host startup on configured signing errors
- [x] Config error surfacing in auth validation path

## Planned / Not Yet Implemented

- [ ] Transport encryption (TLS/mTLS or equivalent confidentiality)
- [ ] Replay resistance/session binding for signed manifests
- [ ] `launcher-v2.sqlite` read-only fallback for playset extraction
- [ ] Resume support for interrupted multi-file transfer sessions
- [ ] Optional cleanup of empty directories after orphan deletions

## Deferred by Design

- [ ] Base game file tree sync (`G`) (intentionally out of scope)
- [ ] rsync engine integration (current architecture intentionally uses direct HTTP pull)

## Tracking

- Gaps and issues are documented in [`docs/gap-findings.md`](gap-findings.md).
