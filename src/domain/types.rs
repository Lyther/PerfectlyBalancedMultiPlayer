use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MANIFEST_VERSION: u32 = 1;
pub const BLAKE3_HEX_LEN: usize = 64;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModId(pub String);

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelativePath(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blake3Hash(pub String);

impl Blake3Hash {
    pub fn validate(s: &str) -> Result<Self, String> {
        if s.len() != BLAKE3_HEX_LEN || !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "Invalid BLAKE3 hash: expected {} hex chars",
                BLAKE3_HEX_LEN
            ));
        }
        Ok(Blake3Hash(s.to_owned()))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncTarget {
    Workshop,
    LocalMod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub generated_at: String,
    /// Path (e.g. "workshop/1234567890/common/buildings.txt") -> BLAKE3 hex. BTreeMap for canonical serialization when signing.
    pub files: std::collections::BTreeMap<String, String>,
    pub load_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedManifest {
    pub manifest: Manifest,
    pub signature: String,
}

impl Manifest {
    pub fn validate_hashes(&self) -> Result<(), String> {
        for (path, h) in &self.files {
            Blake3Hash::validate(h).map_err(|e| format!("{}: {}", path, e))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct StellarisPaths {
    pub game_path: PathBuf,
    pub workshop_path: PathBuf,
    pub user_data_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadOrder {
    pub mods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SMMSConfig {
    pub stellaris: Option<StellarisConfig>,
    pub host: Option<HostConfig>,
    #[serde(default)]
    pub hosts: std::collections::HashMap<String, HostEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StellarisConfig {
    pub game_path: Option<String>,
    pub workshop_path: Option<String>,
    pub user_data_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    pub port: u16,
    pub signing_key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEntry {
    pub public_key: String,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            port: 8730,
            signing_key_path: None,
        }
    }
}
