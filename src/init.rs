use crate::config;
use crate::domain::{HostConfig, PathResolver, SMMSConfig, StellarisConfig};
use crate::path_resolver::SteamPathResolver;
use std::fs;

pub fn run_init() -> Result<(), String> {
    let paths = SteamPathResolver::new()
        .resolve()
        .map_err(|e| format!("Path resolution failed: {}", e))?;

    let config_path = config::config_path().ok_or("Could not determine config directory")?;
    let config_dir = config_path.parent().ok_or("Invalid config path")?;
    fs::create_dir_all(config_dir).map_err(|e| format!("Could not create config dir: {}", e))?;

    let config = SMMSConfig {
        stellaris: Some(StellarisConfig {
            game_path: Some(paths.game_path.to_string_lossy().to_string()),
            workshop_path: Some(paths.workshop_path.to_string_lossy().to_string()),
            user_data_path: Some(paths.user_data_path.to_string_lossy().to_string()),
        }),
        host: Some(HostConfig::default()),
        hosts: std::collections::HashMap::new(),
    };

    let content = toml::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&config_path, content).map_err(|e| format!("Could not write config: {}", e))?;

    eprintln!("✓ Stellaris at {}", paths.game_path.display());
    eprintln!("✓ Workshop at {}", paths.workshop_path.display());
    eprintln!("✓ Config written to {}", config_path.display());
    Ok(())
}
