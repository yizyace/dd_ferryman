use std::net::SocketAddr;
use std::process::Command;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use tracing::info;

use crate::{dns, paths, process, proxy, server, tls};

use super::{DNS_ADDR, HTTPS_ADDR, RESOLVER_PATH};

pub async fn run(foreground: bool) -> Result<()> {
    if let Some(pid) = process::read_pid()? {
        if process::is_running(pid) {
            bail!("dd-ferryman is already running (PID: {pid})");
        }
        process::remove_pid()?;
    }

    first_run_setup()?;

    if foreground {
        run_foreground().await
    } else {
        daemonize()
    }
}

fn first_run_setup() -> Result<()> {
    paths::ensure_dirs()?;

    if !tls::ca::ca_exists()? {
        info!("generating CA certificate");
        tls::ca::load_or_create_ca()?;
    }

    if !ca_is_trusted()? {
        let cert_pem = std::fs::read_to_string(paths::ca_cert_path()?)
            .context("failed to read CA cert for trust")?;
        trust_ca(&cert_pem)?;
    }

    if !std::path::Path::new(RESOLVER_PATH).exists() {
        write_resolver_file()?;
    }

    Ok(())
}

fn ca_is_trusted() -> Result<bool> {
    let output = Command::new("security")
        .args(["dump-trust-settings", "-d"])
        .output()
        .context("failed to query trust settings")?;

    if !output.status.success() {
        return Ok(false);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse the structured output to find our CA with an explicit SSL policy.
    // Format: "Cert N: <name>" followed by indented trust settings blocks.
    let mut in_our_cert = false;
    for line in stdout.lines() {
        if line.contains("dd-ferryman Local CA") {
            in_our_cert = true;
        } else if in_our_cert {
            if line.contains("Policy OID") && line.contains("SSL") {
                return Ok(true);
            }
            // Reached the next cert entry — ours had no SSL policy
            if line.starts_with("Cert ") {
                return Ok(false);
            }
        }
    }

    Ok(false)
}

fn trust_ca(cert_pem: &str) -> Result<()> {
    let cert_path = paths::ca_cert_path()?;

    if !cert_path.exists() {
        std::fs::write(&cert_path, cert_pem).context("failed to write CA cert for trust")?;
    }

    info!("trusting CA certificate in macOS keychain");
    let status = Command::new("security")
        .args([
            "add-trusted-cert",
            "-d",
            "-k",
            "/Library/Keychains/System.keychain",
        ])
        .arg(&cert_path)
        .status()
        .context("failed to run security command")?;

    if !status.success() {
        bail!("failed to trust CA certificate (are you running with sudo?)");
    }

    Ok(())
}

fn write_resolver_file() -> Result<()> {
    info!("writing {RESOLVER_PATH}");

    if let Some(parent) = std::path::Path::new(RESOLVER_PATH).parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    std::fs::write(RESOLVER_PATH, "nameserver 127.0.0.1\nport 9253\n")
        .with_context(|| format!("failed to write {RESOLVER_PATH} (are you running with sudo?)"))?;

    Ok(())
}

async fn run_foreground() -> Result<()> {
    let pid = std::process::id();
    process::write_pid(pid)?;

    info!(pid, "dd-ferryman starting in foreground");

    let shutdown = async {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to register SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("failed to register SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => info!("received SIGTERM"),
            _ = sigint.recv() => info!("received SIGINT"),
        }
    };

    let ca = tls::ca::load_or_create_ca()?;
    let cert_resolver = Arc::new(tls::resolver::CertResolver::new(ca));
    let proxy_state = proxy::ProxyState::new(paths::apps_dir()?);

    let dns_addr: SocketAddr = DNS_ADDR.parse().context("invalid DNS address")?;
    let https_addr: SocketAddr = HTTPS_ADDR.parse().context("invalid HTTPS address")?;

    let dns_handle = tokio::spawn(dns::run_dns_server(dns_addr));
    let https_handle = tokio::spawn(server::run_https_server(
        https_addr,
        cert_resolver,
        proxy_state,
    ));

    tokio::select! {
        result = dns_handle => {
            result?.context("DNS server failed")?;
        }
        result = https_handle => {
            result?.context("HTTPS server failed")?;
        }
        () = shutdown => {
            info!("shutting down");
        }
    }

    process::remove_pid()?;
    info!("dd-ferryman stopped");
    Ok(())
}

fn daemonize() -> Result<()> {
    let exe = std::env::current_exe().context("failed to get current executable path")?;

    let log_path = paths::log_path()?;

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("failed to open log file: {}", log_path.display()))?;

    let log_file_err = log_file
        .try_clone()
        .context("failed to clone log file handle")?;

    let child = Command::new(exe)
        .args(["start", "--foreground"])
        .stdout(log_file)
        .stderr(log_file_err)
        .stdin(std::process::Stdio::null())
        .spawn()
        .context("failed to spawn daemon process")?;

    let pid = child.id();
    println!("dd-ferryman started (PID: {pid})");
    println!("Logs: {}", log_path.display());

    Ok(())
}
