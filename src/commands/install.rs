use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::{LAUNCHD_LABEL, PLIST_PATH};
use crate::process;

pub fn run() -> Result<()> {
    super::start::first_run_setup()?;

    // Stop any manually-started instance
    if let Some(pid) = process::read_pid()? {
        if process::is_running(pid) {
            println!("stopping existing dd-ferryman (PID: {pid})");
            let status = Command::new("kill")
                .arg(pid.to_string())
                .status()
                .context("failed to send SIGTERM")?;
            if !status.success() {
                bail!("failed to stop running dd-ferryman (PID: {pid})");
            }
            for _ in 0..20 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if !process::is_running(pid) {
                    break;
                }
            }
        }
        process::remove_pid()?;
    }

    // Unload existing plist if re-installing
    if Path::new(PLIST_PATH).exists() {
        let _ = Command::new("launchctl")
            .args(["unload", PLIST_PATH])
            .status();
    }

    let exe = std::env::current_exe().context("failed to get current executable path")?;
    let home = dirs::home_dir().context("could not determine home directory")?;
    let log_path = crate::paths::log_path()?;

    let plist = generate_plist(
        &exe.to_string_lossy(),
        &home.to_string_lossy(),
        &log_path.to_string_lossy(),
    );

    std::fs::write(PLIST_PATH, &plist)
        .with_context(|| format!("failed to write {PLIST_PATH} (are you running with sudo?)"))?;

    let status = Command::new("launchctl")
        .args(["load", PLIST_PATH])
        .status()
        .context("failed to run launchctl load")?;

    if !status.success() {
        bail!("launchctl load failed");
    }

    println!("dd-ferryman launchd service installed");
    println!("  Plist:  {PLIST_PATH}");
    println!("  Label:  {LAUNCHD_LABEL}");
    println!("  Logs:   {}", log_path.display());
    println!("\nThe service will start at boot and restart if it crashes.");

    Ok(())
}

fn generate_plist(exe_path: &str, home_dir: &str, log_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LAUNCHD_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_path}</string>
        <string>start</string>
        <string>--foreground</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>EnvironmentVariables</key>
    <dict>
        <key>HOME</key>
        <string>{home_dir}</string>
    </dict>
    <key>StandardOutPath</key>
    <string>{log_path}</string>
    <key>StandardErrorPath</key>
    <string>{log_path}</string>
</dict>
</plist>
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_contains_required_keys() {
        let plist = generate_plist(
            "/usr/local/bin/dd-ferryman",
            "/Users/testuser",
            "/Users/testuser/.dd-ferryman/ferryman.log",
        );

        assert!(plist.contains("<string>com.dd-ferryman</string>"));
        assert!(plist.contains("<string>/usr/local/bin/dd-ferryman</string>"));
        assert!(plist.contains("<string>start</string>"));
        assert!(plist.contains("<string>--foreground</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<true/>"));
        assert!(plist.contains("<key>KeepAlive</key>"));
        assert!(plist.contains("<string>/Users/testuser</string>"));
        assert!(plist.contains("<string>/Users/testuser/.dd-ferryman/ferryman.log</string>"));
    }

    #[test]
    fn plist_is_valid_xml_structure() {
        let plist = generate_plist("/bin/test", "/root", "/tmp/test.log");

        assert!(plist.starts_with("<?xml version=\"1.0\""));
        assert!(plist.contains("<!DOCTYPE plist"));
        assert!(plist.contains("<plist version=\"1.0\">"));
        assert!(plist.ends_with("</plist>\n"));
    }
}
