use anyhow::Result;
use chrono::{DateTime, Local};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    Directory,
    File,
    Symlink { target: Option<PathBuf> },
    Special,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub kind: EntryKind,
    pub size: Option<u64>,
    pub modified: Option<DateTime<Local>>,
    pub permissions: Option<u32>,
    pub is_hidden: bool,
    pub is_executable: bool,
    pub extension: Option<String>,
}

impl Entry {
    pub fn from_path(path: &Path) -> Result<Self> {
        let metadata = fs::symlink_metadata(path)?;
        let ft = metadata.file_type();

        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let is_hidden = name.starts_with('.');

        let kind = if ft.is_symlink() {
            let target = fs::read_link(path).ok();
            EntryKind::Symlink { target }
        } else if ft.is_dir() {
            EntryKind::Directory
        } else if ft.is_file() {
            EntryKind::File
        } else {
            EntryKind::Special
        };

        let size = if ft.is_file() { Some(metadata.len()) } else { None };

        let modified = metadata.modified().ok().and_then(|t| {
            let duration = t.duration_since(std::time::UNIX_EPOCH).ok()?;
            DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
        }).map(|utc: DateTime<chrono::Utc>| utc.with_timezone(&chrono::Local));

        #[cfg(unix)]
        let (permissions, is_executable) = {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            (Some(mode), mode & 0o111 != 0)
        };
        #[cfg(not(unix))]
        let (permissions, is_executable) = (None, false);

        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase());

        Ok(Self {
            path: path.to_path_buf(),
            name,
            kind,
            size,
            modified,
            permissions,
            is_hidden,
            is_executable,
            extension,
        })
    }

    pub fn is_dir(&self) -> bool {
        matches!(self.kind, EntryKind::Directory)
    }

    pub fn is_archive(&self) -> bool {
        matches!(
            self.extension.as_deref(),
            Some("zip" | "tar" | "gz" | "bz2" | "xz" | "zst" | "7z" | "rar")
        )
    }

    pub fn is_media(&self) -> bool {
        matches!(
            self.extension.as_deref(),
            Some(
                "mp4" | "mkv" | "avi" | "mov" | "mp3" | "flac" | "ogg" | "wav"
                    | "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg"
            )
        )
    }
}

pub fn read_dir(path: &Path, show_hidden: bool) -> Result<Vec<Entry>> {
    let mut entries: Vec<Entry> = fs::read_dir(path)?
        .filter_map(|r| r.ok())
        .filter_map(|de| Entry::from_path(&de.path()).ok())
        .filter(|e| show_hidden || !e.is_hidden)
        .collect();

    entries.sort_unstable_by(|a, b| {
        let da = a.is_dir();
        let db = b.is_dir();
        db.cmp(&da).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}
