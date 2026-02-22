# SMMS (Stellaris Multiplayer Mod Sync)

SMMS is a small Rust CLI tool that syncs Stellaris mod state from a host player to clients before multiplayer sessions.

It is built for a practical MVP workflow:

- host serves a manifest and file blobs
- clients pull, diff, verify, and apply
- load order is written via `dlc_load.json`
- `.mod` descriptor `path=` values are rewritten on clients

## MVP Scope (Current)

Implemented now:

- Active playset sync (Workshop + local mods referenced by load order)
- BLAKE3-based diff and verification
- Orphan file cleanup (to remove stale/ghost files)
- `smms verify` (no file writes)
- Optional backup during fetch
- Optional manifest signing (Ed25519) with host key pinning on clients
- Windows + Linux path detection, with config overrides

Out of scope for this MVP:

- TLS/SSH transport encryption
- `launcher-v2.sqlite` read fallback
- Relay/storage mode
- Base game file tree sync
- rsync integration

## Requirements

- Rust toolchain (`cargo`) to build from source
- Stellaris installed (Windows or Linux)
- Steam mod folders accessible on host/client machines
- Network reachability from clients to host (LAN, Tailscale, ZeroTier, etc.)

## Build

```bash
cargo build --release
```

Binary path:

- `target/release/smms` (Linux)
- `target/release/smms.exe` (Windows)

Optional shortcuts:

```bash
make build
make test
make lint
```

## Quick Start

### 1) Initialize on each machine

```bash
smms init
```

This writes `config.toml`:

- Linux: `~/.config/smms/config.toml`
- Windows: `%APPDATA%\\smms\\config.toml`

### 2) Host starts server

```bash
smms serve
# or:
smms serve --port 8730
```

### 3) Client syncs from host

```bash
smms fetch <host-ip-or-name>
```

Useful flags:

- `--no-launch`: sync only, do not launch game
- `--backup`: backup files before overwrite
- `--port <port>`: override host port
- `--allow-empty-manifest`: dangerous override (only for explicit recovery scenarios)

### 4) Client verify-only mode

```bash
smms verify <host-ip-or-name>
```

## Manifest Signing (Recommended on Untrusted Networks)

### Generate host keypair

```bash
smms gen-keypair
```

This writes a host key (default under config directory) and prints config snippets.

### Host config example

```toml
[host]
port = 8730
signing_key_path = "C:/Users/you/AppData/Roaming/smms/host.key"
```

### Client config example

```toml
[hosts."100.101.102.103"]
public_key = "BASE64_ED25519_PUBLIC_KEY"
```

Behavior:

- if client has a pinned key for a host, unsigned manifests are rejected
- if no pinned key is configured, unsigned mode is allowed

## Command Reference

```text
smms init
smms gen-keypair
smms serve [--port <u16>]
smms fetch <host> [--port <u16>] [--no-launch] [--backup] [--allow-empty-manifest]
smms verify <host> [--port <u16>]
```

## Notes

- Transport is plain HTTP. Signing protects authenticity/integrity, not confidentiality.
- For remote sessions, prefer private overlays (Tailscale/ZeroTier).
- If autodetection fails, run `smms init` and manually edit the generated paths in config.

## Documentation

- Architecture: `docs/architecture.md`
- Gap/risk summary: `docs/gap-findings.md`
- API schema: `docs/api/openapi.yaml`
- Design bible: `docs/context/concept-zero.md`
