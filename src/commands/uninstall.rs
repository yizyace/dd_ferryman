use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;

use super::{PLIST_PATH, RESOLVER_PATH};
use crate::{paths, process};

#[derive(Debug, PartialEq, Eq)]
enum CleanupStep {
    UnloadLaunchd,
    StopProcess(u32),
    RemovePidFile,
    RemoveResolver,
    RemoveCaCert(PathBuf),
    RemoveDataDir(PathBuf),
}

fn detect_cleanup(
    plist_exists: bool,
    running_pid: Option<u32>,
    resolver_exists: bool,
    ca_cert_path: Option<PathBuf>,
    data_dir: Option<PathBuf>,
) -> Vec<CleanupStep> {
    let mut steps = Vec::new();

    if plist_exists {
        steps.push(CleanupStep::UnloadLaunchd);
    }
    if let Some(pid) = running_pid {
        steps.push(CleanupStep::StopProcess(pid));
        steps.push(CleanupStep::RemovePidFile);
    }
    if resolver_exists {
        steps.push(CleanupStep::RemoveResolver);
    }
    if let Some(path) = ca_cert_path {
        steps.push(CleanupStep::RemoveCaCert(path));
    }
    if let Some(path) = data_dir {
        steps.push(CleanupStep::RemoveDataDir(path));
    }

    steps
}

pub fn run() -> Result<()> {
    let ca_cert_path = paths::ca_cert_path()?;
    let data_dir = paths::data_dir()?;
    let pid_file_pid = process::read_pid()?;

    let running_pid = pid_file_pid.filter(|&pid| process::is_running(pid));

    let steps = detect_cleanup(
        Path::new(PLIST_PATH).exists(),
        running_pid,
        Path::new(RESOLVER_PATH).exists(),
        ca_cert_path.exists().then_some(ca_cert_path),
        data_dir.exists().then_some(data_dir),
    );

    if steps.is_empty() {
        println!("dd-ferryman is not installed");
        return Ok(());
    }

    for step in &steps {
        execute_step(step);
    }

    Ok(())
}

fn execute_step(step: &CleanupStep) {
    match step {
        CleanupStep::UnloadLaunchd => {
            let _ = Command::new("launchctl")
                .args(["unload", PLIST_PATH])
                .status();
            let _ = std::fs::remove_file(PLIST_PATH);
            println!("Removed dd-ferryman from automatically running");
        }
        CleanupStep::StopProcess(pid) => {
            let _ = Command::new("kill").arg(pid.to_string()).status();
            for _ in 0..20 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if !process::is_running(*pid) {
                    break;
                }
            }
            println!("Stopped dd-ferryman (PID: {pid})");
        }
        CleanupStep::RemovePidFile => {
            let _ = process::remove_pid();
        }
        CleanupStep::RemoveResolver => {
            let _ = std::fs::remove_file(RESOLVER_PATH);
            println!("Removed domain 'test'");
        }
        CleanupStep::RemoveCaCert(path) => {
            let _ = Command::new("security")
                .args(["remove-trusted-cert", "-d"])
                .arg(path)
                .status();
            println!("Removed dd-ferryman CA cert from macOS keychain");
        }
        CleanupStep::RemoveDataDir(path) => {
            let _ = std::fs::remove_dir_all(path);
            println!("Removed {}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_plan_nothing_installed() {
        let steps = detect_cleanup(false, None, false, None, None);

        assert!(steps.is_empty());
    }

    #[test]
    fn cleanup_plan_full_install() {
        let cert = PathBuf::from("/home/test/.dd-ferryman/ca/ca.crt");
        let dir = PathBuf::from("/home/test/.dd-ferryman");

        let steps = detect_cleanup(
            true,
            Some(1234),
            true,
            Some(cert.clone()),
            Some(dir.clone()),
        );

        assert_eq!(
            steps,
            vec![
                CleanupStep::UnloadLaunchd,
                CleanupStep::StopProcess(1234),
                CleanupStep::RemovePidFile,
                CleanupStep::RemoveResolver,
                CleanupStep::RemoveCaCert(cert),
                CleanupStep::RemoveDataDir(dir),
            ]
        );
    }

    #[test]
    fn cleanup_plan_launchd_only() {
        let steps = detect_cleanup(true, None, false, None, None);

        assert_eq!(steps, vec![CleanupStep::UnloadLaunchd]);
    }

    #[test]
    fn cleanup_plan_manual_start_only() {
        let dir = PathBuf::from("/home/test/.dd-ferryman");

        let steps = detect_cleanup(false, Some(5678), false, None, Some(dir.clone()));

        assert_eq!(
            steps,
            vec![
                CleanupStep::StopProcess(5678),
                CleanupStep::RemovePidFile,
                CleanupStep::RemoveDataDir(dir),
            ]
        );
    }
}
