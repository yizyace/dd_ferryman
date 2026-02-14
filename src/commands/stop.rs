use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::process;

pub fn run() -> Result<()> {
    let Some(pid) = process::read_pid()? else {
        println!("dd-ferryman is not running");
        return Ok(());
    };

    if !process::is_running(pid) {
        process::remove_pid()?;
        println!("dd-ferryman is not running (stale PID file removed)");
        return Ok(());
    }

    let status = Command::new("kill")
        .args([&pid.to_string()])
        .status()
        .context("failed to send SIGTERM")?;

    if !status.success() {
        bail!("failed to stop dd-ferryman (PID: {pid})");
    }

    // Wait briefly for process to exit
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if !process::is_running(pid) {
            break;
        }
    }

    process::remove_pid()?;
    println!("dd-ferryman stopped");
    Ok(())
}
