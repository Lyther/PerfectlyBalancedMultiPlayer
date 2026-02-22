use std::path::PathBuf;

fn is_safe_manifest_path(p: &str) -> bool {
    !p.contains("..") && !p.contains('\\') && !p.starts_with('/')
}

fn is_safe_suffix(suffix: &str) -> bool {
    suffix
        .split('/')
        .all(|c| c != "." && c != ".." && !c.is_empty())
}

/// Maps manifest paths (e.g. "workshop/123/common/foo.txt") to filesystem paths.
#[derive(Clone)]
pub struct FileBackend {
    prefix_to_path: Vec<(String, PathBuf)>,
}

impl FileBackend {
    pub fn new(mut entries: Vec<(String, PathBuf)>) -> Self {
        entries.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Self {
            prefix_to_path: entries,
        }
    }

    pub fn resolve(&self, manifest_path: &str) -> Option<PathBuf> {
        let p = self.resolve_path(manifest_path)?;
        if p.exists() {
            Some(p)
        } else {
            None
        }
    }

    /// Returns local path for manifest path (for writing); does not check existence.
    /// Rejects paths with traversal (..) or Windows backslash to prevent escape.
    pub fn resolve_path(&self, manifest_path: &str) -> Option<PathBuf> {
        if !is_safe_manifest_path(manifest_path) {
            return None;
        }
        for (prefix, base) in &self.prefix_to_path {
            let suffix = if manifest_path == prefix {
                ""
            } else if manifest_path.starts_with(&format!("{}/", prefix)) {
                &manifest_path[prefix.len() + 1..]
            } else {
                continue;
            };
            if !suffix.is_empty() && !is_safe_suffix(suffix) {
                continue;
            }
            let resolved = if suffix.is_empty() {
                base.clone()
            } else {
                base.join(suffix)
            };
            if resolved.strip_prefix(base).is_err() {
                continue;
            }
            return Some(resolved);
        }
        None
    }

    pub fn iter_bases(&self) -> impl Iterator<Item = &(String, PathBuf)> {
        self.prefix_to_path.iter()
    }
}
