use std::path::Path;

use anyhow::Result;

use super::{DNS_ADDR, HTTPS_ADDR, PLIST_PATH, RESOLVER_PATH};
use crate::process;

pub fn run() -> Result<()> {
    let Some(pid) = process::read_pid()? else {
        println!("dd-ferryman is not running");
        return Ok(());
    };

    if process::is_running(pid) {
        let resolver_exists = Path::new(RESOLVER_PATH).exists();
        let launchd_installed = Path::new(PLIST_PATH).exists();
        print!(
            "{}",
            format_running(pid, resolver_exists, launchd_installed)
        );
    } else {
        process::remove_pid()?;
        println!("dd-ferryman is not running (stale PID file removed)");
    }

    Ok(())
}

fn format_running(pid: u32, resolver_configured: bool, launchd_installed: bool) -> String {
    let resolver_status = if resolver_configured {
        "configured"
    } else {
        "not configured"
    };

    let launchd_status = if launchd_installed {
        "installed"
    } else {
        "not installed"
    };

    format!(
        "dd-ferryman is running (PID: {pid})\n  \
         DNS:      {DNS_ADDR}\n  \
         HTTPS:    {HTTPS_ADDR}\n  \
         Resolver: {RESOLVER_PATH} ({resolver_status})\n  \
         Launchd:  {launchd_status}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_running_with_resolver() {
        let output = format_running(12345, true, false);
        assert!(output.contains("PID: 12345"));
        assert!(output.contains("DNS:      127.0.0.1:9253"));
        assert!(output.contains("HTTPS:    0.0.0.0:443"));
        assert!(output.contains("/etc/resolver/test (configured)"));
        assert!(output.contains("Launchd:  not installed"));
    }

    #[test]
    fn format_running_without_resolver() {
        let output = format_running(99, false, false);
        assert!(output.contains("PID: 99"));
        assert!(output.contains("(not configured)"));
    }

    #[test]
    fn format_running_with_launchd() {
        let output = format_running(42, true, true);
        assert!(output.contains("Launchd:  installed"));
    }

    #[test]
    fn format_running_without_launchd() {
        let output = format_running(42, true, false);
        assert!(output.contains("Launchd:  not installed"));
    }
}
