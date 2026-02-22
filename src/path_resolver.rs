use crate::domain::{PathResolverError, StellarisPaths};
use keyvalues_parser::Vdf;
use std::path::{Path, PathBuf};

const STELLARIS_APP_ID: u32 = 281990;

pub struct SteamPathResolver;

impl SteamPathResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SteamPathResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::domain::PathResolver for SteamPathResolver {
    fn resolve(&self) -> Result<StellarisPaths, PathResolverError> {
        if let Some(paths) = crate::config::paths_from_config() {
            return Ok(paths);
        }
        let steam_root = find_steam_root()?;
        let library_paths = collect_library_paths(&steam_root)?;
        let (game_path, workshop_path) =
            find_stellaris_in_libraries(&library_paths).ok_or_else(|| {
                PathResolverError::StellarisNotFound(
                    "Stellaris not found in any Steam library".to_string(),
                )
            })?;
        let user_data_path = resolve_user_data_path()?;
        Ok(StellarisPaths {
            game_path,
            workshop_path,
            user_data_path,
        })
    }
}

fn find_steam_root() -> Result<PathBuf, PathResolverError> {
    #[cfg(windows)]
    {
        let reg_path = r"SOFTWARE\WOW6432Node\Valve\Steam";
        if let Ok(path) = read_steam_path_from_registry(reg_path) {
            if path.join("steamapps").exists() {
                return Ok(path);
            }
        }
        let default = PathBuf::from(r"C:\Program Files (x86)\Steam");
        if default.join("steamapps").exists() {
            return Ok(default);
        }
    }

    #[cfg(unix)]
    {
        let candidates = [
            dirs::data_local_dir()
                .map(|p| p.join("Steam"))
                .filter(|p| p.join("steamapps").exists()),
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".steam/steam")),
        ];
        for p in candidates.into_iter().flatten() {
            if p.join("steamapps").exists() {
                return Ok(p);
            }
        }
    }

    Err(PathResolverError::SteamNotFound)
}

#[cfg(windows)]
fn read_steam_path_from_registry(_key: &str) -> Result<PathBuf, PathResolverError> {
    winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
        .open_subkey(_key)
        .map_err(|_| PathResolverError::SteamNotFound)
        .and_then(|key| {
            key.get_value::<String, &str>("InstallPath")
                .map(PathBuf::from)
                .map_err(|_| PathResolverError::SteamNotFound)
        })
}

fn collect_library_paths(steam_root: &Path) -> Result<Vec<PathBuf>, PathResolverError> {
    let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
    let content =
        std::fs::read_to_string(&vdf_path).map_err(|_| PathResolverError::SteamNotFound)?;
    let vdf = Vdf::parse(&content).map_err(|_| PathResolverError::SteamNotFound)?;
    let obj = vdf
        .value
        .get_obj()
        .ok_or(PathResolverError::SteamNotFound)?;

    let mut paths = vec![steam_root.to_path_buf()];
    for (key, values) in obj.iter() {
        if key.parse::<u32>().is_ok() {
            for v in values {
                if let Some(inner) = v.get_obj() {
                    if let Some(path_vals) = inner.get("path") {
                        for pv in path_vals {
                            if let Some(s) = pv.get_str() {
                                let p = PathBuf::from(s.replace("\\\\", "\\"));
                                if p.join("steamapps").exists() && !paths.contains(&p) {
                                    paths.push(p);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(paths)
}

fn find_stellaris_in_libraries(library_paths: &[PathBuf]) -> Option<(PathBuf, PathBuf)> {
    for lib in library_paths {
        let game = lib.join("steamapps/common/Stellaris");
        let workshop = lib
            .join("steamapps/workshop/content")
            .join(STELLARIS_APP_ID.to_string());
        if game.join("stellaris.exe").exists() || game.join("stellaris").exists() {
            return Some((game, workshop));
        }
    }
    None
}

fn resolve_user_data_path() -> Result<PathBuf, PathResolverError> {
    #[cfg(windows)]
    {
        let docs = dirs::document_dir().ok_or_else(|| {
            PathResolverError::StellarisNotFound("Documents folder not found".to_string())
        })?;
        Ok(docs.join("Paradox Interactive/Stellaris"))
    }

    #[cfg(unix)]
    {
        let base = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".local/share"))
                    .unwrap_or_else(|_| PathBuf::from(".local/share"))
            });
        Ok(base.join("Paradox Interactive/Stellaris"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_libraryfolders_extracts_paths() {
        let vdf = r#"
"libraryfolders"
{
    "path"		"C:\\Program Files (x86)\\Steam"
    "0"
    {
        "path"		"C:\\Program Files (x86)\\Steam"
    }
    "1"
    {
        "path"		"D:\\SteamLibrary"
    }
}
"#;
        let parsed = Vdf::parse(vdf).unwrap();
        let obj = parsed.value.get_obj().unwrap();
        let mut paths = vec![];
        for (key, values) in obj.iter() {
            if key.parse::<u32>().is_ok() {
                for v in values {
                    if let Some(inner) = v.get_obj() {
                        if let Some(path_vals) = inner.get("path") {
                            for pv in path_vals {
                                if let Some(s) = pv.get_str() {
                                    paths.push(s.replace("\\\\", "\\"));
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(paths.contains(&r"C:\Program Files (x86)\Steam".to_string()));
        assert!(paths.contains(&r"D:\SteamLibrary".to_string()));
    }
}
