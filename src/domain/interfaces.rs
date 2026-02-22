use super::{LoadOrder, Manifest, StellarisPaths};

pub trait PathResolver: Send + Sync {
    fn resolve(&self) -> Result<StellarisPaths, PathResolverError>;
}

#[derive(Debug, thiserror::Error)]
pub enum PathResolverError {
    #[error("Steam not found")]
    SteamNotFound,
    #[error("Stellaris not found: {0}")]
    StellarisNotFound(String),
}

pub trait PlaysetExtractor: Send + Sync {
    fn active_playset(&self, paths: &StellarisPaths) -> Result<LoadOrder, PlaysetError>;
}

#[derive(Debug, thiserror::Error)]
pub enum PlaysetError {
    #[error("dlc_load.json not found or invalid")]
    DlcLoadNotFound,
    #[error("launcher-v2.sqlite read failed: {0}")]
    LauncherDb(String),
}

pub trait ManifestGenerator: Send + Sync {
    fn generate(
        &self,
        paths: &StellarisPaths,
        load_order: &LoadOrder,
    ) -> Result<Manifest, ManifestError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("mod path not found: {0}")]
    ModPathNotFound(String),
    #[error("hash failed: {0}")]
    HashFailed(String),
}

pub trait DescriptorRewriter: Send + Sync {
    fn rewrite_path(&self, content: &str, paths: &StellarisPaths) -> String;
}
