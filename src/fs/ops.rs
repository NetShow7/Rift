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
