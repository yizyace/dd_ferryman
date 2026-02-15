use std::path::Path;

use anyhow::Result;

use super::{DNS_ADDR, HTTPS_ADDR, RESOLVER_PATH};
use crate::process;

pub fn run() -> Result<()> {
    let Some(pid) = process::read_pid()? else {
        println!("dd-ferryman is not running");
        return Ok(());
    };

    if process::is_running(pid) {
        let resolver_exists = Path::new(RESOLVER_PATH).exists();
        print!("{}", format_running(pid, resolver_exists));
    } else {
        process::remove_pid()?;
        println!("dd-ferryman is not running (stale PID file removed)");
    }

    Ok(())
}

fn format_running(pid: u32, resolver_configured: bool) -> String {
    let resolver_status = if resolver_configured {
        "configured"
    } else {
        "not configured"
    };

    format!(
        "dd-ferryman is running (PID: {pid})\n  \
         DNS:      {DNS_ADDR}\n  \
         HTTPS:    {HTTPS_ADDR}\n  \
         Resolver: {RESOLVER_PATH} ({resolver_status})\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_running_with_resolver() {
        let output = format_running(12345, true);
        assert!(output.contains("PID: 12345"));
        assert!(output.contains("DNS:      127.0.0.1:9253"));
        assert!(output.contains("HTTPS:    0.0.0.0:443"));
        assert!(output.contains("/etc/resolver/test (configured)"));
    }

    #[test]
    fn format_running_without_resolver() {
        let output = format_running(99, false);
        assert!(output.contains("PID: 99"));
        assert!(output.contains("(not configured)"));
    }
}
