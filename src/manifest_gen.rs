use crate::domain::{LoadOrder, Manifest, ManifestError, StellarisPaths};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const IGNORE_NAMES: &[&str] = &[".DS_Store", "Thumbs.db"];

pub struct Blake3ManifestGenerator;

impl Blake3ManifestGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Blake3ManifestGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::domain::ManifestGenerator for Blake3ManifestGenerator {
    fn generate(
        &self,
        paths: &StellarisPaths,
        load_order: &LoadOrder,
    ) -> Result<Manifest, ManifestError> {
        let mut files = BTreeMap::new();
        for mod_ref in &load_order.mods {
            let (prefix, content_path) = resolve_mod_path(paths, mod_ref)
                .ok_or_else(|| ManifestError::ModPathNotFound(mod_ref.clone()))?;
            hash_directory(&content_path, &prefix, &mut files)?;
        }
        Ok(Manifest {
            version: 1,
            generated_at: chrono::Utc::now().to_rfc3339(),
            files,
            load_order: load_order.mods.clone(),
        })
    }
}

pub fn build_file_backend(
    paths: &StellarisPaths,
    load_order: &LoadOrder,
) -> crate::file_backend::FileBackend {
    let mut entries = Vec::new();
    for mod_ref in &load_order.mods {
        if let Some((prefix, content_path)) = resolve_mod_path_unchecked(paths, mod_ref) {
            entries.push((prefix, content_path));
        }
    }
    crate::file_backend::FileBackend::new(entries)
}

fn resolve_mod_path(paths: &StellarisPaths, mod_ref: &str) -> Option<(String, std::path::PathBuf)> {
    let (prefix, content) = resolve_mod_path_unchecked(paths, mod_ref)?;
    if content.exists() {
        Some((prefix, content))
    } else {
        None
    }
}

// FIXED: shared strict load_order validation used by both manifest generation and client-side application paths.
pub(crate) fn is_valid_mod_ref(mod_ref: &str) -> bool {
    let name = match mod_ref
        .strip_prefix("mod/")
        .and_then(|s| s.strip_suffix(".mod"))
    {
        Some(n) => n,
        None => return false,
    };
    if name.is_empty()
        || name.contains("..")
        || name.contains('/')
        || name.contains('\\')
        || name.contains(':')
    {
        return false;
    }
    if let Some(id) = name.strip_prefix("ugc_") {
        return !id.is_empty() && id.chars().all(|c| c.is_ascii_digit());
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

/// Returns (prefix, path) for a mod ref; path may not exist (for client fetch).
pub(crate) fn resolve_mod_path_unchecked(
    paths: &StellarisPaths,
    mod_ref: &str,
) -> Option<(String, std::path::PathBuf)> {
    if !is_valid_mod_ref(mod_ref) {
        return None;
    }
    let name = mod_ref.strip_prefix("mod/")?.strip_suffix(".mod")?;
    if let Some(id) = name.strip_prefix("ugc_") {
        let content = paths.workshop_path.join(id);
        return Some((format!("workshop/{}", id), content));
    }
    let content = paths.user_data_path.join("mod").join(name);
    Some((format!("local/{}", name), content))
}

fn hash_directory(
    dir: &Path,
    prefix: &str,
    out: &mut BTreeMap<String, String>,
) -> Result<(), ManifestError> {
    let meta = fs::symlink_metadata(dir).map_err(|e| ManifestError::HashFailed(e.to_string()))?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| ManifestError::HashFailed(e.to_string()))? {
        let entry = entry.map_err(|e| ManifestError::HashFailed(e.to_string()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if IGNORE_NAMES.contains(&name.as_str()) {
            continue;
        }
        let rel = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", prefix, name)
        };
        let meta =
            fs::symlink_metadata(&path).map_err(|e| ManifestError::HashFailed(e.to_string()))?;
        if meta.is_dir() && !meta.file_type().is_symlink() {
            hash_directory(&path, &rel, out)?;
        } else if meta.is_file() && !meta.file_type().is_symlink() {
            let data = fs::read(&path).map_err(|e| ManifestError::HashFailed(e.to_string()))?;
            let hash = blake3::hash(&data);
            out.insert(rel, hash.to_hex().to_string());
        }
    }
    Ok(())
}
