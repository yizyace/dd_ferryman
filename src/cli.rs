use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dd-ferryman",
    about = "Local DNS + HTTPS for .test domains",
    long_about = "dd-ferryman runs a local DNS and HTTPS server so that .test domains \
                  resolve and serve trusted TLS on your Mac — no /etc/hosts editing, \
                  no self-signed cert warnings."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start DNS and HTTPS servers in the background
    #[command(long_about = "Start DNS and HTTPS servers in the background.\n\n\
                      On first run, generates a local CA, trusts it in the macOS \
                      keychain, and writes /etc/resolver/test. Subsequent runs \
                      reuse the existing CA.\n\n\
                      Requires sudo for port 443 and keychain/resolver setup.")]
    Start {
        /// Run in foreground instead of daemonizing
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the running servers and free all ports
    #[command(long_about = "Stop the running servers and free all ports.\n\n\
                      Sends SIGTERM to the daemon and removes the PID file. \
                      If dd-ferryman is not running, reports that and exits cleanly.")]
    Stop,
    /// Check if servers are running and healthy
    #[command(long_about = "Check if servers are running and healthy.\n\n\
                      Reports the PID, listening ports, and whether the \
                      macOS resolver is configured for .test domains.")]
    Status,
    /// Install launchd service for automatic startup
    #[command(long_about = "Install a launchd service so dd-ferryman starts \
                      automatically at boot and restarts if it crashes.\n\n\
                      Creates a LaunchDaemon plist, loads it via launchctl, \
                      and runs first-time setup if needed.\n\n\
                      Requires sudo.")]
    Install,
    /// Uninstall launchd service
    #[command(long_about = "Uninstall the launchd service and stop dd-ferryman.\n\n\
                      Removes the LaunchDaemon plist and unloads the service. \
                      After uninstalling, dd-ferryman will no longer start \
                      at boot or restart automatically.")]
    Uninstall,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn help_contains_key_phrases() {
        let mut buf = Vec::new();
        Cli::command().write_long_help(&mut buf).unwrap();
        let help = String::from_utf8(buf).unwrap();

        assert!(help.contains(".test domains"));
        assert!(help.contains("start"));
        assert!(help.contains("stop"));
        assert!(help.contains("status"));
        assert!(help.contains("install"));
        assert!(help.contains("uninstall"));
    }

    #[test]
    fn start_help_mentions_background() {
        let mut buf = Vec::new();
        Cli::command()
            .find_subcommand("start")
            .unwrap()
            .clone()
            .write_long_help(&mut buf)
            .unwrap();
        let help = String::from_utf8(buf).unwrap();

        assert!(help.contains("background"));
        assert!(help.contains("--foreground"));
    }

    #[test]
    fn stop_help_mentions_ports() {
        let mut buf = Vec::new();
        Cli::command()
            .find_subcommand("stop")
            .unwrap()
            .clone()
            .write_long_help(&mut buf)
            .unwrap();
        let help = String::from_utf8(buf).unwrap();

        assert!(help.contains("free all ports"));
    }

    #[test]
    fn status_help_mentions_healthy() {
        let mut buf = Vec::new();
        Cli::command()
            .find_subcommand("status")
            .unwrap()
            .clone()
            .write_long_help(&mut buf)
            .unwrap();
        let help = String::from_utf8(buf).unwrap();

        assert!(help.contains("running and healthy"));
    }

    #[test]
    fn install_help_mentions_launchd() {
        let mut buf = Vec::new();
        Cli::command()
            .find_subcommand("install")
            .unwrap()
            .clone()
            .write_long_help(&mut buf)
            .unwrap();
        let help = String::from_utf8(buf).unwrap();

        assert!(help.contains("launchd"));
        assert!(help.contains("boot"));
    }

    #[test]
    fn uninstall_help_mentions_launchd() {
        let mut buf = Vec::new();
        Cli::command()
            .find_subcommand("uninstall")
            .unwrap()
            .clone()
            .write_long_help(&mut buf)
            .unwrap();
        let help = String::from_utf8(buf).unwrap();

        assert!(help.contains("launchd"));
    }
}
