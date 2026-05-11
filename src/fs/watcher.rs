use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use tokio::sync::mpsc;

pub struct FsWatcher {
    _watcher: RecommendedWatcher,
    pub rx: mpsc::Receiver<PathBuf>,
}

impl FsWatcher {
    pub fn new(path: &PathBuf) -> Result<Self> {
        let (tx, rx) = mpsc::channel(64);

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                for p in event.paths {
                    if let Some(parent) = p.parent() {
                        let _ = tx.try_send(parent.to_path_buf());
                    }
                }
            }
        })?;

        watcher.watch(path, RecursiveMode::NonRecursive)
            .map_err(|e| anyhow::anyhow!("watch error: {}", e))?;

        Ok(Self { _watcher: watcher, rx })
    }
}
