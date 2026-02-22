use crate::domain::{LoadOrder, PlaysetError, StellarisPaths};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
struct DlcLoadJson {
    #[serde(default)]
    enabled_mods: Vec<String>,
    #[serde(default)]
    disabled_dlcs: Vec<String>,
}

pub fn write_dlc_load(paths: &StellarisPaths, load_order: &LoadOrder) -> Result<(), String> {
    fs::create_dir_all(&paths.user_data_path).map_err(|e| e.to_string())?;
    let dlc_path = paths.user_data_path.join("dlc_load.json");
    let disabled_dlcs = if dlc_path.exists() {
        if let Ok(content) = fs::read_to_string(&dlc_path) {
            serde_json::from_str::<DlcLoadJson>(&content)
                .map(|j| j.disabled_dlcs)
                .unwrap_or_default()
        } else {
            vec![]
        }
    } else {
        vec![]
    };
    let json = DlcLoadJson {
        enabled_mods: load_order.mods.clone(),
        disabled_dlcs,
    };
    let content = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    fs::write(&dlc_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

pub struct DlcLoadPlaysetExtractor;

impl DlcLoadPlaysetExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DlcLoadPlaysetExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::domain::PlaysetExtractor for DlcLoadPlaysetExtractor {
    fn active_playset(&self, paths: &StellarisPaths) -> Result<LoadOrder, PlaysetError> {
        let dlc_path = paths.user_data_path.join("dlc_load.json");
        if dlc_path.exists() {
            let content =
                fs::read_to_string(&dlc_path).map_err(|_| PlaysetError::DlcLoadNotFound)?;
            let parsed: DlcLoadJson =
                serde_json::from_str(&content).map_err(|_| PlaysetError::DlcLoadNotFound)?;
            if !parsed.enabled_mods.is_empty() {
                return Ok(LoadOrder {
                    mods: parsed.enabled_mods,
                });
            }
        }
        fallback_mod_list(paths)
    }
}

fn fallback_mod_list(paths: &StellarisPaths) -> Result<LoadOrder, PlaysetError> {
    let mod_dir = paths.user_data_path.join("mod");
    if !mod_dir.exists() {
        return Ok(LoadOrder { mods: vec![] });
    }
    let mut mods: Vec<String> = fs::read_dir(&mod_dir)
        .map_err(|_| PlaysetError::DlcLoadNotFound)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "mod"))
        .map(|e| format!("mod/{}", e.file_name().to_string_lossy()))
        .collect();
    mods.sort();
    Ok(LoadOrder { mods })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::PlaysetExtractor;

    fn temp_paths() -> (tempfile::TempDir, StellarisPaths) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let paths = StellarisPaths {
            game_path: root.join("game"),
            workshop_path: root.join("workshop"),
            user_data_path: root.join("user"),
        };
        std::fs::create_dir_all(&paths.user_data_path).unwrap();
        (tmp, paths)
    }

    #[test]
    fn write_dlc_load_creates_file() {
        let (_tmp, paths) = temp_paths();
        let load_order = LoadOrder {
            mods: vec!["mod/ugc_123.mod".into(), "mod/foo.mod".into()],
        };
        write_dlc_load(&paths, &load_order).unwrap();
        let content = std::fs::read_to_string(paths.user_data_path.join("dlc_load.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        let mods = parsed["enabled_mods"].as_array().unwrap();
        assert_eq!(
            mods.iter().map(|v| v.as_str().unwrap()).collect::<Vec<_>>(),
            vec!["mod/ugc_123.mod", "mod/foo.mod"]
        );
    }

    #[test]
    fn active_playset_reads_dlc_load() {
        let (_tmp, paths) = temp_paths();
        let dlc = paths.user_data_path.join("dlc_load.json");
        std::fs::write(
            &dlc,
            r#"{"enabled_mods": ["mod/ugc_1.mod"], "disabled_dlcs": []}"#,
        )
        .unwrap();
        let extractor = DlcLoadPlaysetExtractor::new();
        let order = extractor.active_playset(&paths).unwrap();
        assert_eq!(order.mods, vec!["mod/ugc_1.mod"]);
    }

    #[test]
    fn active_playset_fallback_when_empty() {
        let (_tmp, paths) = temp_paths();
        std::fs::write(
            paths.user_data_path.join("dlc_load.json"),
            r#"{"enabled_mods": [], "disabled_dlcs": []}"#,
        )
        .unwrap();
        let extractor = DlcLoadPlaysetExtractor::new();
        let order = extractor.active_playset(&paths).unwrap();
        assert!(order.mods.is_empty());
    }
}
