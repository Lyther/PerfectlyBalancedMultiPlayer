use crate::config;
use crate::descriptor_rewriter::ModDescriptorRewriter;
use crate::domain::{DescriptorRewriter, Manifest, PathResolver, SignedManifest, MANIFEST_VERSION};
use crate::launcher;
use crate::manifest_gen::build_file_backend;
use crate::path_resolver::SteamPathResolver;
use crate::playset;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

async fn fetch_and_verify_manifest(
    client: &reqwest::Client,
    base_url: &str,
    host: &str,
) -> Result<Manifest, String> {
    let body: serde_json::Value = client
        .get(format!("{}/manifest", base_url))
        .send()
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Manifest fetch failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Invalid manifest: {}", e))?;

    let manifest = if body.get("signature").and_then(|v| v.as_str()).is_some() {
        let signed: SignedManifest =
            serde_json::from_value(body).map_err(|e| format!("Invalid signed manifest: {}", e))?;
        let pub_key = config::host_public_key_for_auth(host.trim())
            .map_err(|e| format!("Config error (cannot verify signed manifest): {}", e))?
            .ok_or_else(|| {
                "Host sent signed manifest but no public key configured. \
                 Add hosts.<host>.public_key to config for authenticity."
                    .to_string()
            })?;
        crate::signing::verify_manifest(&signed.manifest, &signed.signature, &pub_key)
            .map_err(|e| format!("Manifest verification failed: {}", e))?;
        signed.manifest
    } else {
        let manifest: Manifest =
            serde_json::from_value(body).map_err(|e| format!("Invalid manifest: {}", e))?;
        let has_key = config::host_public_key_for_auth(host.trim())
            .map_err(|e| format!("Config error (cannot check auth): {}", e))?;
        if has_key.is_some() {
            return Err(
                "Host public key configured but host sent unsigned manifest. \
                 Configure host signing_key_path on server for authenticity."
                    .to_string(),
            );
        }
        manifest
    };

    manifest
        .validate_hashes()
        .map_err(|e| format!("Invalid manifest hashes: {}", e))?;
    Ok(manifest)
}

pub fn verify_downloaded_file_hash(bytes: &[u8], expected_hash: &str) -> Result<(), String> {
    let actual = blake3::hash(bytes).to_hex().to_string();
    if actual != expected_hash {
        return Err(format!(
            "Hash mismatch: expected {}, got {}",
            expected_hash, actual
        ));
    }
    Ok(())
}

pub fn verify_downloaded_hash(bytes: &[u8], expected_hash: &str, path: &str) -> Result<(), String> {
    verify_downloaded_file_hash(bytes, expected_hash).map_err(|e| format!("{} (path: {})", e, path))
}

pub fn validate_manifest_for_fetch(
    manifest: &Manifest,
    allow_empty_manifest: bool,
) -> Result<(), String> {
    // FIXED: reject malformed/unsafe manifests before any filesystem write/delete operations.
    if manifest.version != MANIFEST_VERSION {
        return Err(format!(
            "Unsupported manifest version {} (expected {})",
            manifest.version, MANIFEST_VERSION
        ));
    }
    for mod_ref in &manifest.load_order {
        if !crate::manifest_gen::is_valid_mod_ref(mod_ref) {
            return Err(format!(
                "Invalid mod reference in manifest load_order: {}",
                mod_ref
            ));
        }
    }
    if manifest.files.is_empty() && !manifest.load_order.is_empty() && !allow_empty_manifest {
        return Err(
            "Host manifest has load_order but no files — possible host error. \
             Refusing to sync (would delete client mods). Use --allow-empty-manifest to override."
                .to_string(),
        );
    }
    Ok(())
}

fn resolve_port(override_port: Option<u16>) -> u16 {
    override_port.unwrap_or_else(crate::config::port_from_config)
}

fn descriptor_file_name_for_mod_ref(mod_ref: &str) -> Option<String> {
    if !crate::manifest_gen::is_valid_mod_ref(mod_ref) {
        return None;
    }
    mod_ref.strip_prefix("mod/").map(str::to_owned)
}

pub async fn verify(host: &str, port_override: Option<u16>) -> Result<(), String> {
    let port = resolve_port(port_override);
    let base_url = format!("http://{}:{}", host.trim(), port);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let manifest = fetch_and_verify_manifest(&client, &base_url, host).await?;
    validate_manifest_for_fetch(&manifest, true)?;

    let paths = SteamPathResolver::new()
        .resolve()
        .map_err(|e| format!("Path resolution: {}", e))?;

    let load_order = crate::domain::LoadOrder {
        mods: manifest.load_order.clone(),
    };
    let backend = build_file_backend(&paths, &load_order);

    if !manifest.files.is_empty() && backend.iter_bases().next().is_none() {
        return Err(
            "Manifest has files but no mods in load_order; cannot resolve paths".to_string(),
        );
    }

    let mut missing = Vec::new();
    let mut mismatched = Vec::new();

    for (manifest_path, expected_hash) in &manifest.files {
        match backend.resolve_path(manifest_path) {
            Some(local_path) if local_path.is_file() => {
                if let Ok(data) = std::fs::read(&local_path) {
                    let actual = blake3::hash(&data).to_hex().to_string();
                    if &actual != expected_hash {
                        mismatched.push(manifest_path.clone());
                    }
                } else {
                    missing.push(manifest_path.clone());
                }
            }
            _ => missing.push(manifest_path.clone()),
        }
    }

    if missing.is_empty() && mismatched.is_empty() {
        eprintln!(
            "✓ Verification PASSED — all {} files match host",
            manifest.files.len()
        );
        return Ok(());
    }

    eprintln!("✗ Verification FAILED");
    if !missing.is_empty() {
        eprintln!("  Missing ({}):", missing.len());
        for p in missing.iter().take(10) {
            eprintln!("    {}", p);
        }
        if missing.len() > 10 {
            eprintln!("    ... and {} more", missing.len() - 10);
        }
    }
    if !mismatched.is_empty() {
        eprintln!("  Mismatched ({}):", mismatched.len());
        for p in mismatched.iter().take(10) {
            eprintln!("    {}", p);
        }
        if mismatched.len() > 10 {
            eprintln!("    ... and {} more", mismatched.len() - 10);
        }
    }
    Err(format!(
        "{} missing, {} mismatched — run `smms fetch {}` to sync",
        missing.len(),
        mismatched.len(),
        host.trim()
    ))
}

fn atomic_write(path: &std::path::Path, data: &[u8]) -> Result<(), std::io::Error> {
    // FIXED: write to a unique temp file, fsync contents, and clean up temp on rename failure.
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("No parent directory for path {}", path.display()),
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("smms-file");
    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let tmp = parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        unique
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)?;
    file.write_all(data)?;
    file.sync_all()?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

fn backup_dir(paths: &crate::domain::StellarisPaths) -> std::path::PathBuf {
    paths
        .user_data_path
        .join(".smms-backup")
        .join(chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string())
}

pub async fn fetch(
    host: &str,
    no_launch: bool,
    backup: bool,
    allow_empty_manifest: bool,
    port_override: Option<u16>,
) -> Result<(), String> {
    let port = resolve_port(port_override);
    let base_url = format!("http://{}:{}", host.trim(), port);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let manifest = fetch_and_verify_manifest(&client, &base_url, host).await?;
    validate_manifest_for_fetch(&manifest, allow_empty_manifest)?;

    let paths = SteamPathResolver::new()
        .resolve()
        .map_err(|e| format!("Path resolution: {}", e))?;

    let load_order = crate::domain::LoadOrder {
        mods: manifest.load_order.clone(),
    };
    let backend = build_file_backend(&paths, &load_order);

    if !manifest.files.is_empty() && backend.iter_bases().next().is_none() {
        return Err(
            "Manifest has files but no mods in load_order; cannot resolve paths".to_string(),
        );
    }

    let mut to_fetch = Vec::new();
    let mut to_delete = Vec::new();
    let mut skipped = 0u64;

    for (manifest_path, expected_hash) in &manifest.files {
        if let Some(local_path) = backend.resolve_path(manifest_path) {
            if local_path.is_file() {
                if let Ok(data) = std::fs::read(&local_path) {
                    let actual = blake3::hash(&data).to_hex().to_string();
                    if &actual == expected_hash {
                        continue;
                    }
                }
            }
        } else {
            skipped += 1;
            continue;
        }
        to_fetch.push(manifest_path.clone());
    }

    if skipped > 0 {
        eprintln!("⚠ Skipped {} files (unresolvable paths)", skipped);
    }

    for (prefix, base) in backend.iter_bases() {
        if base.is_dir() {
            collect_orphans(base, base, prefix, &manifest.files, &mut to_delete);
        }
    }

    let pb = ProgressBar::new(to_fetch.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{bar:40}] {pos}/{len} files")
            .unwrap(),
    );

    let backup_base = if backup {
        let b = backup_dir(&paths);
        std::fs::create_dir_all(&b).map_err(|e| e.to_string())?;
        eprintln!("✓ Backup to {}", b.display());
        Some(b)
    } else {
        None
    };

    for manifest_path in &to_fetch {
        let url = format!("{}/file/{}", base_url, manifest_path.replace('\\', "/"));
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!(
                "Failed to fetch {}: {}",
                manifest_path,
                resp.status()
            ));
        }
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        let expected_hash = manifest
            .files
            .get(manifest_path)
            .ok_or_else(|| format!("Manifest path {} not in manifest", manifest_path))?;
        verify_downloaded_hash(&bytes, expected_hash, manifest_path)?;
        if let Some(local_path) = backend.resolve_path(manifest_path) {
            if let (Some(ref backup_path), true) = (&backup_base, local_path.is_file()) {
                let dest = backup_path.join(manifest_path);
                if let Some(p) = dest.parent() {
                    if std::fs::create_dir_all(p).is_ok() {
                        if std::fs::copy(&local_path, &dest).is_err() {
                            eprintln!("⚠ Backup failed: {}", manifest_path);
                        }
                    } else {
                        eprintln!("⚠ Backup failed: could not create {}", p.display());
                    }
                }
            }
            if let Some(parent) = local_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Create directory {}: {}", parent.display(), e))?;
            }
            atomic_write(&local_path, &bytes)
                .map_err(|e| format!("Write file {}: {}", local_path.display(), e))?;
        }
        pb.inc(1);
    }
    pb.finish_with_message("done");

    eprintln!("✓ Fetched {} files", to_fetch.len());
    if !to_delete.is_empty() {
        eprintln!("✓ Deleted {} orphan files", to_delete.len());
        for p in to_delete {
            // FIXED: re-validate delete targets at delete-time to reduce race/data-loss risk.
            if !backend
                .iter_bases()
                .any(|(_, base)| p.strip_prefix(base).is_ok())
            {
                eprintln!("⚠ Skip unsafe delete target: {}", p.display());
                continue;
            }
            match std::fs::symlink_metadata(&p) {
                Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
                    if let Err(e) = std::fs::remove_file(&p) {
                        eprintln!("⚠ Delete failed {}: {}", p.display(), e);
                    }
                }
                Ok(_) => eprintln!("⚠ Skip non-regular orphan: {}", p.display()),
                Err(e) => eprintln!("⚠ Skip delete {}: {}", p.display(), e),
            }
        }
    }

    let rewriter = ModDescriptorRewriter::new();
    let mod_dir = paths.user_data_path.join("mod");
    for mod_ref in &load_order.mods {
        let Some(name) = descriptor_file_name_for_mod_ref(mod_ref) else {
            eprintln!("⚠ Skip invalid mod descriptor ref: {}", mod_ref);
            continue;
        };
        let desc_path = mod_dir.join(name);
        if desc_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&desc_path) {
                let rewritten = rewriter.rewrite_path(&content, &paths);
                std::fs::write(&desc_path, rewritten).map_err(|e| e.to_string())?;
            }
        }
    }
    eprintln!("✓ Rewrote .mod descriptors");

    playset::write_dlc_load(&paths, &load_order)?;
    eprintln!("✓ Wrote dlc_load.json");

    if !no_launch {
        launcher::spawn_stellaris(&paths)?;
        eprintln!("✓ Launched Stellaris");
    }

    Ok(())
}

fn collect_orphans(
    base: &std::path::Path,
    dir: &std::path::Path,
    prefix: &str,
    manifest_files: &std::collections::BTreeMap<String, String>,
    out: &mut Vec<PathBuf>,
) {
    if std::fs::symlink_metadata(dir)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(true)
    {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let path = e.path();
            if let Ok(meta) = std::fs::symlink_metadata(&path) {
                if meta.is_dir() && !meta.file_type().is_symlink() {
                    collect_orphans(base, &path, prefix, manifest_files, out);
                } else if meta.is_file() && !meta.file_type().is_symlink() {
                    let rel = path
                        .strip_prefix(base)
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_default();
                    let manifest_path = format!("{}/{}", prefix, rel);
                    if !manifest_files.contains_key(&manifest_path) {
                        out.push(path);
                    }
                }
            }
        }
    }
}
