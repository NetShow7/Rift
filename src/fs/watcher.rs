use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

pub struct FsWatcher {
    watcher: RecommendedWatcher,
    pub watched: HashSet<PathBuf>,
    pub rx: mpsc::Receiver<PathBuf>,
}

impl FsWatcher {
    /// Create a new FsWatcher without watching any directory.
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::channel(64);

        let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                for p in event.paths {
                    if let Some(parent) = p.parent() {
                        let _ = tx.try_send(parent.to_path_buf());
                    }
                }
            }
        })?;

        Ok(Self {
            watcher,
            watched: HashSet::new(),
            rx,
        })
    }

    /// Add a directory to watch. No-ops if already watched.
    pub fn add_watch(&mut self, path: &Path) -> Result<()> {
        if self.watched.contains(path) {
            return Ok(());
        }
        self.watcher.watch(path, RecursiveMode::NonRecursive)?;
        self.watched.insert(path.to_path_buf());
        Ok(())
    }

    /// Remove a directory from watching. Silently ignores errors.
    pub fn remove_watch(&mut self, path: &Path) {
        let _ = self.watcher.unwatch(path);
        self.watched.remove(path);
    }

    /// Create a watcher watching a single directory.
    pub fn new_with_path(path: &PathBuf) -> Result<Self> {
        let mut fw = Self::new()?;
        fw.add_watch(path)?;
        Ok(fw)
    }

    /// Return a snapshot of currently watched paths.
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    /// Compute which directories should be watched based on layout.
    /// Returns the unique set of directory paths that need file-system watchers.
    /// Panes: 0 = primary, 1 = secondary, 2 = parent (Miller layout).
    pub fn compute_watched_dirs(
        primary: &Path,
        secondary: Option<&Path>,
        parent: Option<&Path>,
    ) -> HashSet<PathBuf> {
        let mut dirs = HashSet::new();
        dirs.insert(primary.to_path_buf());
        if let Some(s) = secondary {
            dirs.insert(s.to_path_buf());
        }
        if let Some(p) = parent {
            dirs.insert(p.to_path_buf());
        }
        dirs
    }

    /// Route a changed directory to the pane that should be refreshed.
    /// Returns `Some(0)` for primary, `Some(1)` for secondary, `Some(2)` for parent,
    /// or `None` if the path does not match any pane's cwd.
    pub fn find_affected_pane(
        changed_dir: &Path,
        primary: &Path,
        secondary: Option<&Path>,
        parent: Option<&Path>,
    ) -> Option<usize> {
        if changed_dir == primary {
            return Some(0);
        }
        if let Some(s) = secondary {
            if changed_dir == s {
                return Some(1);
            }
        }
        if let Some(p) = parent {
            if changed_dir == p {
                return Some(2);
            }
        }
        None
    }

    #[test]
    fn test_single_layout_watches_only_primary() {
        let dir = tempfile::TempDir::new().unwrap();
        let primary = dir.path();
        let dirs = compute_watched_dirs(primary, None, None);
        assert_eq!(dirs.len(), 1);
        assert!(dirs.contains(primary));
    }

    #[test]
    fn test_dual_layout_watches_primary_and_secondary() {
        let primary_dir = tempfile::TempDir::new().unwrap();
        let secondary_dir = tempfile::TempDir::new().unwrap();
        let dirs = compute_watched_dirs(
            primary_dir.path(),
            Some(secondary_dir.path()),
            None,
        );
        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains(primary_dir.path()));
        assert!(dirs.contains(secondary_dir.path()));
    }

    #[test]
    fn test_miller_layout_watches_primary_and_parent() {
        let primary_dir = tempfile::TempDir::new().unwrap();
        let parent_dir = tempfile::TempDir::new().unwrap();
        let dirs = compute_watched_dirs(
            primary_dir.path(),
            None,
            Some(parent_dir.path()),
        );
        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains(primary_dir.path()));
        assert!(dirs.contains(parent_dir.path()));
    }

    #[test]
    fn test_dedup_same_directory() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path();
        let dirs = compute_watched_dirs(path, Some(path), None);
        assert_eq!(dirs.len(), 1);
        assert!(dirs.contains(path));
    }

    #[test]
    fn test_event_routes_to_correct_pane() {
        let primary_dir = tempfile::TempDir::new().unwrap();
        let secondary_dir = tempfile::TempDir::new().unwrap();
        let primary = primary_dir.path();
        let secondary = secondary_dir.path();

        let result = find_affected_pane(primary, primary, Some(secondary), None);
        assert_eq!(result, Some(0));

        let result = find_affected_pane(secondary, primary, Some(secondary), None);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_event_no_match() {
        let primary_dir = tempfile::TempDir::new().unwrap();
        let unknown_dir = tempfile::TempDir::new().unwrap();
        let result = find_affected_pane(
            unknown_dir.path(),
            primary_dir.path(),
            None,
            None,
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_event_routes_to_parent_pane() {
        let primary_dir = tempfile::TempDir::new().unwrap();
        let parent_dir = tempfile::TempDir::new().unwrap();
        let result = find_affected_pane(
            parent_dir.path(),
            primary_dir.path(),
            None,
            Some(parent_dir.path()),
        );
        assert_eq!(result, Some(2));
    }
}
