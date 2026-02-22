# SMMS Sync Sequence

Client `fetch` flow: retrieve, validate, diff, transfer, apply.

## Diagram

```mermaid
sequenceDiagram
    participant C as Client
    participant H as Host

    C->>H: GET /manifest
    H-->>C: Manifest or SignedManifest

    alt signed payload
        C->>C: Lookup hosts.<host>.public_key
        C->>C: Verify Ed25519 signature
    else unsigned payload
        C->>C: Require no pinned key for host
    end

    C->>C: Validate manifest version + hash format + load_order refs
    Note over C: Diff local BLAKE3 vs manifest

    loop For each missing/mismatched file
        C->>H: GET /file/{path}
        H-->>C: File blob
        C->>C: Verify blob hash
        C->>C: Atomic write to local path
    end

    loop For each local file not in manifest
        C->>C: DELETE regular file (managed roots only)
    end

    C->>C: Rewrite .mod path= for client Steam
    C->>C: Write dlc_load.json
    opt default behavior
        C->>C: Spawn stellaris.exe (unless --no-launch)
    end
```

## Notes

- Host does not push; client pulls only
- Delete step removes ghost/orphan files within managed mod roots
- Transport is HTTP; signing provides authenticity only when keys are configured
