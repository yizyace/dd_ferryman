use std::fs;
use std::process::Command;

use anyhow::{Context, Result};

use crate::paths;

pub fn write_pid(pid: u32) -> Result<()> {
    let path = paths::pid_path()?;
    fs::write(&path, pid.to_string())
        .with_context(|| format!("failed to write PID file: {}", path.display()))?;
    Ok(())
}

pub fn read_pid() -> Result<Option<u32>> {
    let path = paths::pid_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path).context("failed to read PID file")?;
    let pid: u32 = contents.trim().parse().context("invalid PID in file")?;
    Ok(Some(pid))
}

pub fn remove_pid() -> Result<()> {
    let path = paths::pid_path()?;
    if path.exists() {
        fs::remove_file(&path).context("failed to remove PID file")?;
    }
    Ok(())
}

pub fn is_running(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .is_ok_and(|output| output.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_running_detects_current_process() {
        let pid = std::process::id();
        assert!(is_running(pid));
    }

    #[test]
    fn is_running_returns_false_for_nonexistent_pid() {
        assert!(!is_running(4_000_000_000));
    }
}
