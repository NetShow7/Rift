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

    pub fn is_previewable(&self) -> bool {
        if self.extension.is_none() {
            return matches!(
                self.name.as_str(),
                "Makefile" | "Dockerfile" | "Procfile" | "Justfile"
            );
        }
        matches!(
            self.extension.as_deref(),
            Some(
                // Code
                "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "rb" | "java"
                | "c" | "cpp" | "h" | "hpp" | "cs" | "swift" | "kt" | "scala"
                | "php" | "pl" | "lua" | "r" | "dart" | "zig" | "clj" | "ex"
                | "exs" | "erl" | "hs" | "nim" | "odin" | "gleam"
                // Shell & build
                | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd"
                | "cmake" | "gradle" | "starlark" | "bazel" | "mk"
                // Config
                | "toml" | "yaml" | "yml" | "json" | "xml" | "ini" | "cfg"
                | "conf" | "env" | "editorconfig" | "gitignore" | "gitattributes"
                | "dockerignore" | "npmrc" | "prettierrc" | "eslintrc"
                | "hcl" | "terraform" | "lock"
                // Docs
                | "md" | "txt" | "rst" | "tex" | "org" | "adoc" | "mdx"
                // Web
                | "html" | "htm" | "css" | "scss" | "sass" | "less" | "vue"
                | "svelte" | "astro" | "liquid" | "ejs" | "hbs"
                // Other text
                | "log" | "csv" | "tsv" | "diff" | "patch" | "sql" | "graphql"
                | "gql" | "proto" | "nix" | "justfile" | "procfile"
                | "properties" | "desktop" | "service" | "timer"
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn entry_from_path(path: &Path) -> Entry {
        Entry::from_path(path).unwrap()
    }

    #[test]
    fn entry_from_path_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();
        let entry = entry_from_path(&file_path);
        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.kind, EntryKind::File);
        assert_eq!(entry.size, Some(5));
        assert!(!entry.is_hidden);
        assert!(!entry.is_dir());
        assert_eq!(entry.extension.as_deref(), Some("txt"));
    }

    #[test]
    fn entry_from_path_directory() {
        let dir = TempDir::new().unwrap();
        let entry = entry_from_path(dir.path());
        assert_eq!(entry.kind, EntryKind::Directory);
        assert!(entry.is_dir());
        assert_eq!(entry.size, None);
    }

    #[test]
    fn entry_hidden_file() {
        let dir = TempDir::new().unwrap();
        let hidden = dir.path().join(".hidden");
        std::fs::write(&hidden, "secret").unwrap();
        let entry = entry_from_path(&hidden);
        assert!(entry.is_hidden);
    }

    #[test]
    fn entry_is_archive() {
        let dir = TempDir::new().unwrap();
        for ext in &["zip", "tar", "gz", "bz2", "xz", "zst", "7z", "rar"] {
            let f = dir.path().join(format!("archive.{}", ext));
            std::fs::write(&f, "").unwrap();
            let entry = entry_from_path(&f);
            assert!(entry.is_archive(), "Expected .{} to be archive", ext);
        }
    }

    #[test]
    fn entry_is_not_archive() {
        let dir = TempDir::new().unwrap();
        let f = dir.path().join("document.txt");
        std::fs::write(&f, "").unwrap();
        let entry = entry_from_path(&f);
        assert!(!entry.is_archive());
    }

    #[test]
    fn entry_is_media() {
        let dir = TempDir::new().unwrap();
        for ext in &["mp4", "mkv", "avi", "mov", "mp3", "flac", "ogg", "wav",
                      "jpg", "jpeg", "png", "gif", "webp", "svg"] {
            let f = dir.path().join(format!("media.{}", ext));
            std::fs::write(&f, "").unwrap();
            let entry = entry_from_path(&f);
            assert!(entry.is_media(), "Expected .{} to be media", ext);
        }
    }

    #[test]
    fn entry_is_not_media() {
        let dir = TempDir::new().unwrap();
        let f = dir.path().join("notes.txt");
        std::fs::write(&f, "").unwrap();
        let entry = entry_from_path(&f);
        assert!(!entry.is_media());
    }

    #[test]
    fn entry_is_previewable() {
        let dir = TempDir::new().unwrap();
        for ext in &["rs", "py", "js", "ts", "md", "toml", "json", "html",
                      "css", "sh", "txt", "yaml", "xml", "csv", "log", "tex"] {
            let f = dir.path().join(format!("file.{}", ext));
            std::fs::write(&f, "").unwrap();
            let entry = entry_from_path(&f);
            assert!(entry.is_previewable(), "Expected .{} to be previewable", ext);
        }
    }

    #[test]
    fn entry_is_not_previewable() {
        let dir = TempDir::new().unwrap();
        for ext in &["bin", "exe", "dll", "o", "so", "class"] {
            let f = dir.path().join(format!("binary.{}", ext));
            std::fs::write(&f, "").unwrap();
            let entry = entry_from_path(&f);
            assert!(!entry.is_previewable(), "Expected .{} to NOT be previewable", ext);
        }
    }

    #[test]
    fn entry_previewable_special_names() {
        let dir = TempDir::new().unwrap();
        for name in &["Makefile", "Dockerfile", "Procfile", "Justfile"] {
            let f = dir.path().join(name);
            std::fs::write(&f, "").unwrap();
            let entry = entry_from_path(&f);
            assert!(entry.is_previewable(), "Expected {} to be previewable", name);
        }
    }

    #[test]
    fn read_dir_sorts_dirs_first_then_alpha() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("zzz_dir")).unwrap();
        std::fs::create_dir(dir.path().join("aaa_dir")).unwrap();
        std::fs::write(dir.path().join("bbb_file"), "").unwrap();
        std::fs::write(dir.path().join("ccc_file"), "").unwrap();
        let entries = read_dir(dir.path(), true).unwrap();
        assert_eq!(entries.len(), 4);
        assert!(entries[0].is_dir());
        assert!(entries[1].is_dir());
        assert!(!entries[2].is_dir());
        assert!(!entries[3].is_dir());
        assert_eq!(entries[0].name, "aaa_dir");
        assert_eq!(entries[1].name, "zzz_dir");
        assert_eq!(entries[2].name, "bbb_file");
        assert_eq!(entries[3].name, "ccc_file");
    }

    #[test]
    fn read_dir_hides_hidden_without_flag() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("visible"), "").unwrap();
        std::fs::write(dir.path().join(".hidden"), "").unwrap();
        let entries = read_dir(dir.path(), false).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "visible");
    }

    #[test]
    fn read_dir_shows_hidden_with_flag() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("visible"), "").unwrap();
        std::fs::write(dir.path().join(".hidden"), "").unwrap();
        let entries = read_dir(dir.path(), true).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn executable_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let f = dir.path().join("script.sh");
        std::fs::write(&f, "#!/bin/sh").unwrap();
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
        let entry = entry_from_path(&f);
        assert!(entry.is_executable);
    }

    #[cfg(unix)]
    #[test]
    fn entry_symlink() {
        use std::os::unix::fs::symlink;
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        std::fs::write(&target, "target").unwrap();
        symlink(&target, &link).unwrap();
        let entry = entry_from_path(&link);
        assert!(matches!(entry.kind, EntryKind::Symlink { .. }));
    }
}
