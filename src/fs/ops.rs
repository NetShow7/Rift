use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResolution {
    Skip,
    Overwrite,
    Rename(String),
    Abort,
}

#[derive(Debug, Clone)]
pub enum OpResult {
    Success { src: PathBuf, dst: PathBuf },
    Skipped { src: PathBuf },
    Failed  { src: PathBuf, error: String },
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub src: PathBuf,
    pub dst: PathBuf,
}

pub async fn copy_entries(
    sources: &[PathBuf],
    dest_dir: &Path,
    mut resolve: impl FnMut(Conflict) -> ConflictResolution,
) -> Vec<OpResult> {
    let mut results = Vec::new();
    for src in sources {
        let name = src.file_name().unwrap_or_default();
        let dst = dest_dir.join(name);
        let effective_dst = if dst.exists() {
            match resolve(Conflict { src: src.clone(), dst: dst.clone() }) {
                ConflictResolution::Skip      => { results.push(OpResult::Skipped { src: src.clone() }); continue; }
                ConflictResolution::Abort     => break,
                ConflictResolution::Overwrite => dst,
                ConflictResolution::Rename(n) => dest_dir.join(n),
            }
        } else {
            dst
        };
        match fs::metadata(src).await.map(|m| m.is_dir()) {
            Ok(true)  => match copy_dir(src, &effective_dst).await {
                Ok(_)  => results.push(OpResult::Success { src: src.clone(), dst: effective_dst }),
                Err(e) => results.push(OpResult::Failed  { src: src.clone(), error: e.to_string() }),
            },
            _ => match fs::copy(src, &effective_dst).await {
                Ok(_)  => results.push(OpResult::Success { src: src.clone(), dst: effective_dst }),
                Err(e) => results.push(OpResult::Failed  { src: src.clone(), error: e.to_string() }),
            },
        }
    }
    results
}

async fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).await?;
    let mut read_dir = fs::read_dir(src).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let child_src = entry.path();
        let child_dst = dst.join(entry.file_name());
        if child_src.is_dir() {
            Box::pin(copy_dir(&child_src, &child_dst)).await?;
        } else {
            fs::copy(&child_src, &child_dst).await
                .with_context(|| format!("copy {} -> {}", child_src.display(), child_dst.display()))?;
        }
    }
    Ok(())
}

pub async fn move_entries(
    sources: &[PathBuf],
    dest_dir: &Path,
    mut resolve: impl FnMut(Conflict) -> ConflictResolution,
) -> Vec<OpResult> {
    let mut results = Vec::new();
    for src in sources {
        let name = src.file_name().unwrap_or_default();
        let dst = dest_dir.join(name);
        let effective_dst = if dst.exists() {
            match resolve(Conflict { src: src.clone(), dst: dst.clone() }) {
                ConflictResolution::Skip      => { results.push(OpResult::Skipped { src: src.clone() }); continue; }
                ConflictResolution::Abort     => break,
                ConflictResolution::Overwrite => dst,
                ConflictResolution::Rename(n) => dest_dir.join(n),
            }
        } else {
            dst
        };
        match move_single(src, &effective_dst).await {
            Ok(_)  => results.push(OpResult::Success { src: src.clone(), dst: effective_dst }),
            Err(e) => results.push(OpResult::Failed  { src: src.clone(), error: e.to_string() }),
        }
    }
    results
}

async fn move_single(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).await?;
    }
    if fs::rename(src, dst).await.is_ok() {
        return Ok(());
    }
    copy_dir(src, dst).await?;
    delete_path(src).await
}

pub async fn delete_entries(sources: &[PathBuf]) -> Vec<OpResult> {
    let mut results = Vec::new();
    for src in sources {
        match delete_path(src).await {
            Ok(_)  => results.push(OpResult::Success { src: src.clone(), dst: src.clone() }),
            Err(e) => results.push(OpResult::Failed  { src: src.clone(), error: e.to_string() }),
        }
    }
    results
}

async fn delete_path(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path).await
            .with_context(|| format!("delete dir {}", path.display()))
    } else {
        fs::remove_file(path).await
            .with_context(|| format!("delete file {}", path.display()))
    }
}

pub async fn rename_entry(src: &Path, new_name: &str) -> Result<PathBuf> {
    let dst = src.parent().unwrap_or(Path::new(".")).join(new_name);
    fs::rename(src, &dst).await
        .with_context(|| format!("rename {} -> {}", src.display(), dst.display()))?;
    Ok(dst)
}

pub async fn create_file(dir: &Path, name: &str) -> Result<PathBuf> {
    let path = dir.join(name);
    fs::File::create(&path).await
        .with_context(|| format!("create file {}", path.display()))?;
    Ok(path)
}

pub async fn create_dir(dir: &Path, name: &str) -> Result<PathBuf> {
    let path = dir.join(name);
    fs::create_dir_all(&path).await
        .with_context(|| format!("create dir {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn create_file_and_rename() {
        let dir = TempDir::new().unwrap();
        let path = create_file(dir.path(), "test.txt").await.unwrap();
        assert!(path.exists());
        let renamed = rename_entry(&path, "newname.txt").await.unwrap();
        assert!(!path.exists());
        assert!(renamed.exists());
        assert_eq!(renamed.file_name().unwrap(), "newname.txt");
    }

    #[tokio::test]
    async fn create_dir_test() {
        let dir = TempDir::new().unwrap();
        let path = super::create_dir(dir.path(), "subdir").await.unwrap();
        assert!(path.exists());
        assert!(path.is_dir());
    }

    #[tokio::test]
    async fn create_nested_dir_test() {
        let dir = TempDir::new().unwrap();
        let path = super::create_dir(dir.path(), "a/b/c").await.unwrap();
        assert!(path.exists());
        assert!(path.is_dir());
    }

    #[tokio::test]
    async fn copy_file() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        let file = src.path().join("test.txt");
        std::fs::write(&file, "hello").unwrap();
        let results = copy_entries(&[file], dst.path(), |_| ConflictResolution::Overwrite).await;
        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], OpResult::Success { .. }));
        assert!(dst.path().join("test.txt").exists());
        assert_eq!(std::fs::read_to_string(dst.path().join("test.txt")).unwrap(), "hello");
    }

    #[tokio::test]
    async fn copy_directory() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        let sub = src.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("nested.txt"), "content").unwrap();
        let results = copy_entries(&[sub.clone()], dst.path(), |_| ConflictResolution::Overwrite).await;
        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], OpResult::Success { .. }));
        assert!(dst.path().join("subdir").join("nested.txt").exists());
    }

    #[tokio::test]
    async fn move_file() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        let file = src.path().join("moveme.txt");
        std::fs::write(&file, "move me").unwrap();
        let results = move_entries(&[file.clone()], dst.path(), |_| ConflictResolution::Overwrite).await;
        assert_eq!(results.len(), 1);
        assert!(!file.exists());
        assert!(dst.path().join("moveme.txt").exists());
    }

    #[tokio::test]
    async fn delete_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("delete_me.txt");
        std::fs::write(&file, "bye").unwrap();
        let results = delete_entries(&[file.clone()]).await;
        assert_eq!(results.len(), 1);
        assert!(!file.exists());
    }

    #[tokio::test]
    async fn delete_directory() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("nested.txt"), "content").unwrap();
        let results = delete_entries(&[sub.clone()]).await;
        assert_eq!(results.len(), 1);
        assert!(!sub.exists());
    }

    #[tokio::test]
    async fn delete_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("does_not_exist.txt");
        let results = delete_entries(&[file]).await;
        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], OpResult::Failed { .. }));
    }

    #[tokio::test]
    async fn conflict_resolution_skip() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        let file = src.path().join("shared.txt");
        std::fs::write(&file, "original").unwrap();
        std::fs::write(dst.path().join("shared.txt"), "existing").unwrap();
        let results = copy_entries(&[file.clone()], dst.path(), |_| ConflictResolution::Skip).await;
        assert!(matches!(&results[0], OpResult::Skipped { .. }));
        assert_eq!(std::fs::read_to_string(dst.path().join("shared.txt")).unwrap(), "existing");
    }

    #[tokio::test]
    async fn conflict_resolution_overwrite() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        let file = src.path().join("shared.txt");
        std::fs::write(&file, "new content").unwrap();
        std::fs::write(dst.path().join("shared.txt"), "old content").unwrap();
        let results = copy_entries(&[file.clone()], dst.path(), |_| ConflictResolution::Overwrite).await;
        assert!(matches!(&results[0], OpResult::Success { .. }));
        assert_eq!(std::fs::read_to_string(dst.path().join("shared.txt")).unwrap(), "new content");
    }

    #[tokio::test]
    async fn conflict_resolution_rename() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        let file = src.path().join("shared.txt");
        std::fs::write(&file, "original").unwrap();
        std::fs::write(dst.path().join("shared.txt"), "existing").unwrap();
        let results = copy_entries(&[file.clone()], dst.path(), |_| {
            ConflictResolution::Rename("shared_copy.txt".into())
        }).await;
        assert!(matches!(&results[0], OpResult::Success { .. }));
        assert!(dst.path().join("shared_copy.txt").exists());
    }

    #[tokio::test]
    async fn conflict_resolution_abort() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        std::fs::write(src.path().join("a.txt"), "a").unwrap();
        std::fs::write(src.path().join("b.txt"), "b").unwrap();
        std::fs::write(dst.path().join("a.txt"), "existing_a").unwrap();
        let results = copy_entries(
            &[src.path().join("a.txt"), src.path().join("b.txt")],
            dst.path(),
            |_| ConflictResolution::Abort,
        ).await;
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn move_with_conflict_overwrite() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();
        let a = src_dir.path().join("a.txt");
        std::fs::write(&a, "a").unwrap();
        std::fs::write(dst_dir.path().join("a.txt"), "existing").unwrap();
        let results = move_entries(&[a.clone()], dst_dir.path(), |_| ConflictResolution::Overwrite).await;
        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], OpResult::Success { .. }));
        assert!(!a.exists());
        assert_eq!(std::fs::read_to_string(dst_dir.path().join("a.txt")).unwrap(), "a");
    }

    #[tokio::test]
    async fn copy_multiple_files() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        for i in 0..3 {
            std::fs::write(src.path().join(format!("file{}.txt", i)), format!("content{}", i)).unwrap();
        }
        let files: Vec<PathBuf> = (0..3).map(|i| src.path().join(format!("file{}.txt", i))).collect();
        let results = copy_entries(&files, dst.path(), |_| ConflictResolution::Overwrite).await;
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(matches!(r, OpResult::Success { .. }));
        }
    }

    #[tokio::test]
    async fn conflict_resolution_multiple_with_mixed() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        std::fs::write(src.path().join("a.txt"), "a").unwrap();
        std::fs::write(src.path().join("b.txt"), "b").unwrap();
        std::fs::write(dst.path().join("a.txt"), "existing_a").unwrap();
        let mut call_count = 0u32;
        let results = copy_entries(
            &[src.path().join("a.txt"), src.path().join("b.txt")],
            dst.path(),
            |_| {
                call_count += 1;
                if call_count == 1 { ConflictResolution::Skip }
                else { ConflictResolution::Overwrite }
            },
        ).await;
        assert_eq!(results.len(), 2);
        assert!(matches!(&results[0], OpResult::Skipped { .. }));
        assert!(matches!(&results[1], OpResult::Success { .. }));
    }
}
