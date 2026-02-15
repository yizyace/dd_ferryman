use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::paths;

pub fn run(name: &str) -> Result<()> {
    let apps_dir = paths::apps_dir()?;
    let entry = apps_dir.join(name);
    remove_app_entry(&entry, name)?;
    println!("unlinked {name}.test");
    Ok(())
}

fn remove_app_entry(path: &Path, name: &str) -> Result<()> {
    let meta = std::fs::symlink_metadata(path)
        .with_context(|| format!("no app registered for {name}.test"))?;

    if meta.is_dir() && !meta.is_symlink() {
        bail!("refusing to remove directory: {}", path.display());
    }

    std::fs::remove_file(path).with_context(|| format!("failed to remove: {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_app_entry_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("myapp");
        std::fs::write(&path, "3000\n").unwrap();

        remove_app_entry(&path, "myapp").unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn remove_app_entry_removes_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("project");
        std::fs::create_dir(&target).unwrap();

        let link = dir.path().join("myapp");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        remove_app_entry(&link, "myapp").unwrap();
        assert!(!link.exists());
        assert!(target.exists());
    }

    #[test]
    fn remove_app_entry_fails_for_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing");

        let err = remove_app_entry(&path, "missing").unwrap_err();
        assert!(err.to_string().contains("no app registered"));
    }

    #[test]
    fn remove_app_entry_refuses_real_directory() {
        let dir = tempfile::tempdir().unwrap();
        let real_dir = dir.path().join("realdir");
        std::fs::create_dir(&real_dir).unwrap();

        let err = remove_app_entry(&real_dir, "realdir").unwrap_err();
        assert!(err.to_string().contains("refusing to remove directory"));
    }
}
