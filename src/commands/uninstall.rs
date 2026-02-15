use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::PLIST_PATH;
use crate::process;

pub fn run() -> Result<()> {
    if !Path::new(PLIST_PATH).exists() {
        println!("dd-ferryman launchd service is not installed");
        return Ok(());
    }

    let status = Command::new("launchctl")
        .args(["unload", PLIST_PATH])
        .status()
        .context("failed to run launchctl unload")?;

    if !status.success() {
        bail!("launchctl unload failed");
    }

    std::fs::remove_file(PLIST_PATH)
        .with_context(|| format!("failed to remove {PLIST_PATH}"))?;

    process::remove_pid()?;

    println!("dd-ferryman launchd service uninstalled");

    Ok(())
}
