use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use anyhow::{Context, Result};

pub(crate) fn data_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".dd-ferryman"))
}

pub fn ca_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("ca"))
}

pub fn ca_cert_path() -> Result<PathBuf> {
    Ok(ca_dir()?.join("ca.crt"))
}

pub fn ca_key_path() -> Result<PathBuf> {
    Ok(ca_dir()?.join("ca.key"))
}

pub fn apps_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("apps"))
}

pub fn pid_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("ferryman.pid"))
}

pub fn log_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("ferryman.log"))
}

pub fn ensure_dirs() -> Result<()> {
    for dir in [ca_dir()?, apps_dir()?] {
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create directory: {}", dir.display()))?;
    }
    Ok(())
}

/// Chown the data directory and its immediate children to match the real
/// user's home directory ownership.  This fixes the case where `sudo`
/// created the dirs as root but `link`/`unlink` run unprivileged.
///
/// Errors are intentionally ignored — if we aren't root the chown is
/// unnecessary (dirs already belong to us) or impossible anyway.
pub(crate) fn fix_data_dir_ownership() -> Result<()> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    let home_meta = std::fs::metadata(&home)
        .with_context(|| format!("failed to stat home directory: {}", home.display()))?;

    let uid = home_meta.uid();
    let gid = home_meta.gid();

    let data = data_dir()?;
    let _ = std::os::unix::fs::chown(&data, Some(uid), Some(gid));

    for entry in std::fs::read_dir(&data).into_iter().flatten().flatten() {
        let _ = std::os::unix::fs::chown(entry.path(), Some(uid), Some(gid));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_is_under_home() {
        let dir = data_dir().unwrap();
        assert!(dir.ends_with(".dd-ferryman"));
    }

    #[test]
    fn ca_paths_are_under_ca_dir() {
        let ca = ca_dir().unwrap();
        let cert = ca_cert_path().unwrap();
        let key = ca_key_path().unwrap();
        assert!(cert.starts_with(&ca));
        assert!(key.starts_with(&ca));
    }
}
