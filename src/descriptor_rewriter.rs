use crate::domain::{DescriptorRewriter, StellarisPaths};
use regex::Regex;
use std::sync::OnceLock;

static PATH_RE: OnceLock<Regex> = OnceLock::new();

pub struct ModDescriptorRewriter;

impl ModDescriptorRewriter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ModDescriptorRewriter {
    fn default() -> Self {
        Self::new()
    }
}

impl DescriptorRewriter for ModDescriptorRewriter {
    fn rewrite_path(&self, content: &str, paths: &StellarisPaths) -> String {
        let re = PATH_RE.get_or_init(|| Regex::new(r#"path="([^"]*)""#).expect("path= regex"));
        re.replace_all(content, |caps: &regex::Captures<'_>| {
            let old_path = &caps[1];
            let new_path = rewrite_single_path(old_path, paths);
            let s = new_path.to_string_lossy().replace('\\', "/");
            format!(r#"path="{}""#, s)
        })
        .into_owned()
    }
}

fn rewrite_single_path(old: &str, paths: &StellarisPaths) -> std::path::PathBuf {
    let normalized = old.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return paths.workshop_path.clone();
    }
    let last = parts[parts.len() - 1];
    if normalized.contains("workshop/content") && last.chars().all(|c| c.is_ascii_digit()) {
        return paths.workshop_path.join(last);
    }
    if let Some(idx) = parts.iter().position(|&p| p == "mod") {
        if idx + 1 < parts.len() {
            let name = parts[idx + 1];
            return paths.user_data_path.join("mod").join(name);
        }
    }
    paths.workshop_path.join(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_paths() -> StellarisPaths {
        StellarisPaths {
            game_path: PathBuf::from("D:/Steam/steamapps/common/Stellaris"),
            workshop_path: PathBuf::from("D:/Steam/steamapps/workshop/content/281990"),
            user_data_path: PathBuf::from("C:/Users/foo/Documents/Paradox Interactive/Stellaris"),
        }
    }

    #[test]
    fn rewrite_workshop_path() {
        let paths = test_paths();
        let rewriter = ModDescriptorRewriter::new();
        let content = r#"name="Test Mod"
path="C:/Steam/steamapps/workshop/content/281990/1234567890"
version="1.0""#;
        let out = rewriter.rewrite_path(content, &paths);
        assert!(out.contains(r#"path="D:/Steam/steamapps/workshop/content/281990/1234567890""#));
    }

    #[test]
    fn rewrite_local_mod_path() {
        let paths = test_paths();
        let rewriter = ModDescriptorRewriter::new();
        let content = r#"name="My Mod"
path="C:/Users/host/Documents/Paradox Interactive/Stellaris/mod/my_mod"
version="1.0""#;
        let out = rewriter.rewrite_path(content, &paths);
        assert!(out
            .contains(r#"path="C:/Users/foo/Documents/Paradox Interactive/Stellaris/mod/my_mod""#));
    }
}
