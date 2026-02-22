use crate::domain::{SMMSConfig, StellarisPaths};
use std::path::PathBuf;

pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("smms").join("config.toml"))
}

pub fn load_config() -> Option<SMMSConfig> {
    load_config_optional_result().ok().flatten()
}

pub fn load_config_result() -> Result<SMMSConfig, String> {
    // FIXED: differentiate "config missing" from malformed/failed reads so auth code can fail closed on real errors.
    load_config_optional_result()?
        .ok_or_else(|| "Config file not found. Run `smms init` first.".to_string())
}

fn load_config_optional_result() -> Result<Option<SMMSConfig>, String> {
    let path = config_path().ok_or("Could not determine config directory")?;
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!("Config read failed ({}): {}", path.display(), e));
        }
    };
    let parsed = toml::from_str(&content).map_err(|e| format!("Config parse failed: {}", e))?;
    Ok(Some(parsed))
}

pub fn port_from_config() -> u16 {
    load_config()
        .and_then(|c| c.host)
        .map(|h| h.port)
        .unwrap_or(8730)
}

pub fn host_public_key(host: &str) -> Option<String> {
    load_config().and_then(|c| c.hosts.get(host.trim()).map(|e| e.public_key.clone()))
}

pub fn host_public_key_for_auth(host: &str) -> Result<Option<String>, String> {
    Ok(load_config_optional_result()?
        .and_then(|c| c.hosts.get(host.trim()).map(|e| e.public_key.clone())))
}

pub fn signing_key_path() -> Option<std::path::PathBuf> {
    let config = load_config()?;
    let path = config.host.as_ref()?.signing_key_path.as_ref()?;
    Some(std::path::PathBuf::from(path))
}

pub fn signing_key_path_for_auth() -> Result<Option<std::path::PathBuf>, String> {
    // FIXED: host startup must fail on parse/read errors when signing is expected, instead of silently serving unsigned.
    Ok(load_config_optional_result()?.and_then(|c| {
        c.host
            .and_then(|h| h.signing_key_path.map(std::path::PathBuf::from))
    }))
}

pub fn paths_from_config() -> Option<StellarisPaths> {
    let config = load_config()?;
    let stellaris = config.stellaris?;
    let game = stellaris.game_path.map(PathBuf::from)?;
    let workshop = stellaris.workshop_path.map(PathBuf::from)?;
    let user_data = stellaris.user_data_path.map(PathBuf::from)?;
    if game.exists() && workshop.exists() && user_data.exists() {
        Some(StellarisPaths {
            game_path: game,
            workshop_path: workshop,
            user_data_path: user_data,
        })
    } else {
        None
    }
}
