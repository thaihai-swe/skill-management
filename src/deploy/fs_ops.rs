use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn create_symlink(source: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory {:?}", parent))?;
    }

    if target.exists() || is_broken_symlink(target) {
        remove_path(target)?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target).with_context(|| {
            format!("Failed to create symlink from {:?} to {:?}", source, target)
        })?;
    }

    #[cfg(windows)]
    {
        if source.is_dir() {
            // Try directory symlink first
            if let Err(_) = std::os::windows::fs::symlink_dir(source, target) {
                // If developer mode isn't enabled, fallback to copy
                copy_dir_recursive(source, target)?;
            }
        } else {
            if let Err(_) = std::os::windows::fs::symlink_file(source, target) {
                fs::copy(source, target)?;
            }
        }
    }

    Ok(())
}

pub fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    if !source.exists() {
        anyhow::bail!("Source directory does not exist: {:?}", source);
    }

    if target.exists() {
        remove_path(target)?;
    }

    if source.is_file() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, target)?;
        return Ok(());
    }

    fs::create_dir_all(target)?;

    for entry in walkdir::WalkDir::new(source) {
        let entry = entry?;
        let rel_path = entry.path().strip_prefix(source)?;
        let dest_path = target.join(rel_path);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &dest_path)?;
        }
    }

    Ok(())
}

pub fn remove_path(path: &Path) -> Result<()> {
    if is_broken_symlink(path) || path.is_symlink() {
        #[cfg(unix)]
        {
            fs::remove_file(path)?;
        }
        #[cfg(windows)]
        {
            if path.is_dir() {
                fs::remove_dir(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }
    } else if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn is_broken_symlink(path: &Path) -> bool {
    path.is_symlink() && !path.exists()
}

pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory {:?}", parent))?;
    }

    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_path = path.with_extension(format!("tmp.{}.{}", pid, nanos));

    fs::write(&tmp_path, content)
        .with_context(|| format!("Failed to write temporary file {:?}", tmp_path))?;

    #[cfg(unix)]
    {
        fs::rename(&tmp_path, path)
            .with_context(|| format!("Failed to atomically rename {:?} to {:?}", tmp_path, path))?;
    }

    #[cfg(windows)]
    {
        if path.exists() {
            let _ = remove_path(path);
        }
        fs::rename(&tmp_path, path)
            .with_context(|| format!("Failed to atomically rename {:?} to {:?}", tmp_path, path))?;
    }

    Ok(())
}
