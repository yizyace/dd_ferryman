use std::path::PathBuf;

use anyhow::{Context, Result};

fn data_dir() -> Result<PathBuf> {
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
