use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

const FILE_NAME: &str = "favorites.txt";

/// Session names the user has starred.
///
/// Favorites cannot live on [`crate::domain::Session`] alone: every refresh
/// rebuilds that tree from tmux, which knows nothing about them. They are keyed
/// by session name (the stable, user-facing identifier — tmux session ids are
/// recycled across server restarts) and persisted so they survive a restart.
#[derive(Debug, Clone, Default)]
pub struct Favorites {
    names: BTreeSet<String>,
    path: Option<PathBuf>,
}

impl Favorites {
    /// Load from disk. A missing or unreadable file yields an empty set.
    pub fn load() -> Self {
        let path = crate::config::resolve_path(FILE_NAME);
        let names = path
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .map(|contents| {
                contents
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        Self { names, path }
    }

    /// An in-memory set that is never written to disk. For tests and mock mode.
    pub fn ephemeral() -> Self {
        Self {
            names: BTreeSet::new(),
            path: None,
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    /// Toggle `name` and persist. Returns the new state.
    pub fn toggle(&mut self, name: &str) -> bool {
        let added = if self.names.contains(name) {
            self.names.remove(name);
            false
        } else {
            self.names.insert(name.to_string());
            true
        };
        self.save();
        added
    }

    /// Rename in place so a starred session keeps its star. No-op if not starred.
    pub fn rename(&mut self, old: &str, new: &str) {
        if self.names.remove(old) {
            self.names.insert(new.to_string());
            self.save();
        }
    }

    /// Best-effort write. Persistence failures must not interrupt the UI, so
    /// errors are dropped: the worst case is a star that does not outlive the
    /// process, which is exactly the pre-persistence behaviour.
    fn save(&self) {
        let Some(path) = &self.path else { return };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut contents = self.names.iter().cloned().collect::<Vec<_>>().join("\n");
        contents.push('\n');
        let _ = fs::write(path, contents);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_round_trip() {
        let mut favs = Favorites::ephemeral();
        assert!(!favs.contains("work"));
        assert!(favs.toggle("work"));
        assert!(favs.contains("work"));
        assert!(!favs.toggle("work"));
        assert!(!favs.contains("work"));
    }

    #[test]
    fn test_rename_keeps_star() {
        let mut favs = Favorites::ephemeral();
        favs.toggle("old");
        favs.rename("old", "new");
        assert!(!favs.contains("old"));
        assert!(favs.contains("new"));

        // Renaming an unstarred session must not create a star.
        favs.rename("absent", "other");
        assert!(!favs.contains("other"));
    }

    #[test]
    fn test_ephemeral_never_writes() {
        let favs = Favorites::ephemeral();
        assert!(favs.path.is_none());
    }

    #[test]
    fn test_persists_and_reloads_from_disk() {
        let dir = std::env::temp_dir().join(format!("lazytmux_fav_{}", std::process::id()));
        let path = dir.join(FILE_NAME);
        let mut favs = Favorites {
            names: BTreeSet::new(),
            path: Some(path.clone()),
        };
        favs.toggle("work");
        favs.toggle("side project");

        let contents = fs::read_to_string(&path).expect("favorites file should be written");
        let reloaded: BTreeSet<String> = contents
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::to_string)
            .collect();
        assert!(reloaded.contains("work"));
        assert!(reloaded.contains("side project"));

        favs.toggle("work");
        let contents = fs::read_to_string(&path).unwrap();
        assert!(!contents.lines().any(|l| l == "work"));

        let _ = fs::remove_dir_all(&dir);
    }
}
