use crate::domain::Manifest;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::path::Path;

pub fn sign_manifest(manifest: &Manifest, key_path: &Path) -> Result<String, String> {
    let key_bytes = std::fs::read(key_path).map_err(|e| format!("Read signing key: {}", e))?;
    let key = SigningKey::from_bytes(
        key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Signing key must be 32 bytes")?,
    );
    let msg = serde_json::to_vec(manifest).map_err(|e| format!("Serialize manifest: {}", e))?;
    let sig = key.sign(&msg);
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        sig.to_bytes(),
    ))
}

pub fn verify_manifest(
    manifest: &Manifest,
    signature_b64: &str,
    pub_key_b64: &str,
) -> Result<(), String> {
    let sig_bytes =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, signature_b64)
            .map_err(|e| format!("Invalid signature encoding: {}", e))?;
    let sig = Signature::from_bytes(
        sig_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Signature must be 64 bytes")?,
    );
    let pub_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, pub_key_b64)
        .map_err(|e| format!("Invalid public key encoding: {}", e))?;
    let verifying_key = VerifyingKey::from_bytes(
        pub_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Public key must be 32 bytes")?,
    )
    .map_err(|e| format!("Invalid public key: {}", e))?;
    let msg = serde_json::to_vec(manifest).map_err(|e| format!("Serialize manifest: {}", e))?;
    verifying_key
        .verify(&msg, &sig)
        .map_err(|_| "Signature verification failed")?;
    Ok(())
}

pub fn generate_keypair() -> (SigningKey, String) {
    use rand::rngs::OsRng;
    let mut rng = OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();
    let pub_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        verifying_key.to_bytes(),
    );
    (signing_key, pub_b64)
}

pub fn run_gen_keypair() -> Result<(), String> {
    let config_dir = crate::config::config_path()
        .ok_or("Could not determine config directory")?
        .parent()
        .ok_or("Invalid config path")?
        .to_path_buf();
    std::fs::create_dir_all(&config_dir).map_err(|e| format!("Create config dir: {}", e))?;
    let key_path = config_dir.join("host.key");
    let (signing_key, pub_b64) = generate_keypair();
    std::fs::write(&key_path, signing_key.to_bytes())
        .map_err(|e| format!("Write signing key: {}", e))?;
    eprintln!("✓ Signing key written to {}", key_path.display());
    eprintln!("Add to client config [hosts.<host>]:");
    eprintln!("  public_key = \"{}\"", pub_b64);
    eprintln!("Add to host config [host]:");
    eprintln!("  signing_key_path = \"{}\"", key_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn test_manifest() -> Manifest {
        let mut files = BTreeMap::new();
        files.insert("a/1.txt".to_string(), "a".repeat(64));
        files.insert("b/2.txt".to_string(), "b".repeat(64));
        Manifest {
            version: 1,
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            files,
            load_order: vec!["mod/ugc_1.mod".to_string()],
        }
    }

    #[test]
    fn sign_verify_roundtrip() {
        let manifest = test_manifest();
        let (signing_key, pub_b64) = generate_keypair();
        let sig = signing_key.sign(&serde_json::to_vec(&manifest).unwrap());
        let sig_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, sig.to_bytes());
        verify_manifest(&manifest, &sig_b64, &pub_b64).unwrap();
    }

    #[test]
    fn sign_manifest_verify_roundtrip() {
        let manifest = test_manifest();
        let (signing_key, pub_b64) = generate_keypair();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), signing_key.to_bytes()).unwrap();
        let sig_b64 = sign_manifest(&manifest, tmp.path()).unwrap();
        verify_manifest(&manifest, &sig_b64, &pub_b64).unwrap();
    }

    #[test]
    fn verify_rejects_tampered_manifest() {
        let (signing_key, pub_b64) = generate_keypair();
        let manifest = test_manifest();
        let sig = signing_key.sign(&serde_json::to_vec(&manifest).unwrap());
        let sig_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, sig.to_bytes());
        let mut tampered = manifest.clone();
        tampered.files.insert("evil".to_string(), "x".repeat(64));
        assert!(verify_manifest(&tampered, &sig_b64, &pub_b64).is_err());
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let (key1, _) = generate_keypair();
        let (_, pub2_b64) = generate_keypair();
        let manifest = test_manifest();
        let sig = key1.sign(&serde_json::to_vec(&manifest).unwrap());
        let sig_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, sig.to_bytes());
        assert!(verify_manifest(&manifest, &sig_b64, &pub2_b64).is_err());
    }

    #[test]
    fn canonical_serialization_determinism() {
        let manifest = test_manifest();
        let a = serde_json::to_vec(&manifest).unwrap();
        let b = serde_json::to_vec(&manifest).unwrap();
        assert_eq!(a, b, "BTreeMap must serialize identically across calls");
    }
}
