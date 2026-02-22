# SMMS System Overview

High-level components and data flow (as built).

## Diagram

```mermaid
flowchart TB
    subgraph Host["Host Machine"]
        SteamH[Steam + Workshop]
        DocsH[Documents/Paradox]
        HostCfg[config.toml]
        SMMSH[SMMS serve]
        SteamH --> SMMSH
        DocsH --> SMMSH
        HostCfg --> SMMSH
    end

    subgraph Client["Client Machine"]
        SteamC[Steam + Workshop]
        DocsC[Documents/Paradox]
        ClientCfg[config.toml]
        SMMSC[SMMS fetch]
        Game[stellaris.exe]
        SteamC --> SMMSC
        DocsC --> SMMSC
        ClientCfg --> SMMSC
        SMMSC --> Game
    end

    SMMSH -->|HTTP :8730| SMMSC
```

## Components

- **SMMS serve**: path resolver, playset extractor, manifest generator, optional manifest signer, HTTP server (`/manifest`, `/file/*path`)
- **SMMS fetch**: manifest verifier, diff engine, file fetcher, orphan cleanup, descriptor rewriter, `dlc_load.json` writer, optional launcher bypass
- **Steam + Workshop**: `libraryfolders.vdf`, `workshop/content/281990`
- **Documents/Paradox**: `mod/`, `dlc_load.json`
- **config.toml**:
  - host: paths, port, optional `signing_key_path`
  - client: host key pinning via `[hosts.<host>].public_key`
