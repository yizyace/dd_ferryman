use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::paths;
use crate::proxy::PORT_FILENAME;

pub fn run(name: Option<&str>, port: Option<u16>, path: Option<&Path>) -> Result<()> {
    let apps_dir = paths::apps_dir()?;
    std::fs::create_dir_all(&apps_dir)
        .with_context(|| format!("failed to create apps directory: {}", apps_dir.display()))?;

    let target_path = resolve_path(path)?;
    let name = resolve_name(name, &target_path)?;
    validate_name(&name)?;

    if let Some(port) = port {
        write_port_file(&apps_dir, &name, port)?;
        println!("{name}.test -> http://127.0.0.1:{port}");
    } else {
        create_symlink(&apps_dir, &name, &target_path)?;
        println!(
            "{name}.test -> {} (reads {PORT_FILENAME} for port)",
            target_path.display()
        );
    }

    Ok(())
}

fn resolve_name(name: Option<&str>, path: &Path) -> Result<String> {
    if let Some(name) = name {
        return Ok(name.to_owned());
    }
    path.file_name()
        .and_then(|n| n.to_str())
        .map(ToOwned::to_owned)
        .context("could not determine app name from path")
}

fn resolve_path(path: Option<&Path>) -> Result<PathBuf> {
    let raw = match path {
        Some(p) => p.to_owned(),
        None => std::env::current_dir().context("could not determine current directory")?,
    };
    let canonical = std::fs::canonicalize(&raw)
        .with_context(|| format!("path not found: {}", raw.display()))?;
    if !canonical.is_dir() {
        bail!("not a directory: {}", canonical.display());
    }
    Ok(canonical)
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\0') {
        bail!("invalid app name: {name:?}");
    }
    Ok(())
}

fn write_port_file(apps_dir: &Path, name: &str, port: u16) -> Result<()> {
    let path = apps_dir.join(name);
    remove_entry(&path)?;
    std::fs::write(&path, format!("{port}\n"))
        .with_context(|| format!("failed to write port file: {}", path.display()))?;
    Ok(())
}

fn create_symlink(apps_dir: &Path, name: &str, target: &Path) -> Result<()> {
    let link_path = apps_dir.join(name);
    remove_entry(&link_path)?;
    std::os::unix::fs::symlink(target, &link_path)
        .with_context(|| format!("failed to create symlink: {}", link_path.display()))?;
    Ok(())
}

fn remove_entry(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() && !meta.is_symlink() => {
            bail!("refusing to remove directory: {}", path.display());
        }
        Ok(_) => {
            std::fs::remove_file(path)
                .with_context(|| format!("failed to remove: {}", path.display()))?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(e).with_context(|| format!("failed to check: {}", path.display()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_name_accepts_normal_name() {
        assert!(validate_name("myapp").is_ok());
    }

    #[test]
    fn validate_name_rejects_empty() {
        assert!(validate_name("").is_err());
    }

    #[test]
    fn validate_name_rejects_dot() {
        assert!(validate_name(".").is_err());
        assert!(validate_name("..").is_err());
    }

    #[test]
    fn validate_name_rejects_slash() {
        assert!(validate_name("foo/bar").is_err());
    }

    #[test]
    fn resolve_name_uses_explicit_name() {
        let name = resolve_name(Some("api"), Path::new("/some/path")).unwrap();
        assert_eq!(name, "api");
    }

    #[test]
    fn resolve_name_falls_back_to_basename() {
        let name = resolve_name(None, Path::new("/home/user/myapp")).unwrap();
        assert_eq!(name, "myapp");
    }

    #[test]
    fn resolve_name_fails_for_root() {
        assert!(resolve_name(None, Path::new("/")).is_err());
    }

    #[test]
    fn write_port_file_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        write_port_file(dir.path(), "myapp", 3000).unwrap();
        let contents = std::fs::read_to_string(dir.path().join("myapp")).unwrap();
        assert_eq!(contents, "3000\n");
    }

    #[test]
    fn write_port_file_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        write_port_file(dir.path(), "myapp", 3000).unwrap();
        write_port_file(dir.path(), "myapp", 4000).unwrap();
        let contents = std::fs::read_to_string(dir.path().join("myapp")).unwrap();
        assert_eq!(contents, "4000\n");
    }

    #[test]
    fn create_symlink_creates_link() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("project");
        std::fs::create_dir(&target).unwrap();

        let apps = dir.path().join("apps");
        std::fs::create_dir(&apps).unwrap();

        create_symlink(&apps, "myapp", &target).unwrap();
        let link = std::fs::read_link(apps.join("myapp")).unwrap();
        assert_eq!(link, target);
    }

    #[test]
    fn create_symlink_replaces_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("project");
        std::fs::create_dir(&target).unwrap();

        let apps = dir.path().join("apps");
        std::fs::create_dir(&apps).unwrap();
        std::fs::write(apps.join("myapp"), "3000\n").unwrap();

        create_symlink(&apps, "myapp", &target).unwrap();
        let link = std::fs::read_link(apps.join("myapp")).unwrap();
        assert_eq!(link, target);
    }

    #[test]
    fn create_symlink_replaces_existing_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let old_target = dir.path().join("old");
        std::fs::create_dir(&old_target).unwrap();
        let new_target = dir.path().join("new");
        std::fs::create_dir(&new_target).unwrap();

        let apps = dir.path().join("apps");
        std::fs::create_dir(&apps).unwrap();

        create_symlink(&apps, "myapp", &old_target).unwrap();
        create_symlink(&apps, "myapp", &new_target).unwrap();
        let link = std::fs::read_link(apps.join("myapp")).unwrap();
        assert_eq!(link, new_target);
    }

    #[test]
    fn resolve_path_canonicalizes_directory() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_path(Some(dir.path())).unwrap();
        assert!(result.is_absolute());
        assert!(result.is_dir());
    }

    #[test]
    fn resolve_path_rejects_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not_a_dir");
        std::fs::write(&file, "").unwrap();
        assert!(resolve_path(Some(&file)).is_err());
    }

    #[test]
    fn resolve_path_rejects_nonexistent() {
        assert!(resolve_path(Some(Path::new("/nonexistent/path/xyzzy"))).is_err());
    }
}
