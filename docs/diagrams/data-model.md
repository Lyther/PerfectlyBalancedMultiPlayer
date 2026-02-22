# SMMS Data Model

Domain entities and wire format (as built).

## Diagram

```mermaid
erDiagram
    SignedManifest ||--|| Manifest : wraps
    Manifest ||--|{ ManifestFile : contains
    Manifest ||--o{ LoadOrderEntry : has
    StellarisPaths ||--|| Manifest : used_by
    LoadOrder ||--|| Manifest : embedded
    SMMSConfig ||--o| HostConfig : has
    SMMSConfig ||--o{ HostEntry : hosts

    Manifest {
        int version
        string generated_at
        array load_order
    }

    ManifestFile {
        string path "workshop/123/common/foo.txt"
        string blake3 "64 hex chars"
    }

    StellarisPaths {
        path game_path
        path workshop_path
        path user_data_path
    }

    LoadOrder {
        array mods "mod/ugc_123.mod"
    }

    SignedManifest {
        object manifest
        string signature "base64 Ed25519"
    }

    HostConfig {
        int port
        string signing_key_path "optional"
    }

    HostEntry {
        string public_key "base64 Ed25519"
    }
```

## Wire Format (JSON)

Unsigned manifest response:

```json
{
  "version": 1,
  "generated_at": "2026-02-22T15:30:00Z",
  "files": {
    "workshop/1234567890/common/buildings.txt": "af3b...",
    "local/my_mod/common/foo.txt": "b2c1..."
  },
  "load_order": ["mod/ugc_1234567890.mod", "mod/my_mod.mod"]
}
```

Signed manifest response:

```json
{
  "manifest": {
    "version": 1,
    "generated_at": "2026-02-22T15:30:00Z",
    "files": {
      "workshop/1234567890/common/buildings.txt": "af3b...",
      "local/my_mod/common/foo.txt": "b2c1..."
    },
    "load_order": ["mod/ugc_1234567890.mod", "mod/my_mod.mod"]
  },
  "signature": "base64-ed25519-signature"
}
```
