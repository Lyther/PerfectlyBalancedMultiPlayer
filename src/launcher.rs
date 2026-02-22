use crate::domain::StellarisPaths;
use std::process::Command;

pub fn spawn_stellaris(paths: &StellarisPaths) -> Result<(), String> {
    let exe = if cfg!(target_os = "windows") {
        paths.game_path.join("stellaris.exe")
    } else {
        paths.game_path.join("stellaris")
    };
    if !exe.exists() {
        return Err(format!("Game executable not found: {}", exe.display()));
    }
    Command::new(&exe)
        .current_dir(&paths.game_path)
        .spawn()
        .map_err(|e| format!("Failed to spawn stellaris: {}", e))?;
    Ok(())
}
