use std::fs;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use anyhow::{Context, Result};
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair, KeyUsagePurpose};

use crate::paths;

pub struct CaBundle {
    pub cert: rcgen::Certificate,
    pub key_pair: KeyPair,
}

fn ca_params() -> CertificateParams {
    let mut params = CertificateParams::default();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params
        .distinguished_name
        .push(DnType::CommonName, "dd-ferryman Local CA");
    params
        .distinguished_name
        .push(DnType::OrganizationName, "dd-ferryman");

    params.not_before = rcgen::date_time_ymd(2024, 1, 1);
    params.not_after = rcgen::date_time_ymd(2034, 1, 1);

    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];

    params
}

pub fn ca_exists() -> Result<bool> {
    Ok(ca_exists_in(&paths::ca_dir()?))
}

pub fn generate_ca() -> Result<CaBundle> {
    let params = ca_params();
    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;
    Ok(CaBundle { cert, key_pair })
}

pub fn load_or_create_ca() -> Result<CaBundle> {
    let dir = paths::ca_dir()?;
    if ca_exists_in(&dir) {
        load_ca_from(&dir)
    } else {
        paths::ensure_dirs()?;
        let bundle = generate_ca()?;
        save_ca_to(&bundle, &dir)?;
        Ok(bundle)
    }
}

fn ca_exists_in(dir: &Path) -> bool {
    dir.join("ca.crt").exists() && dir.join("ca.key").exists()
}

fn save_ca_to(bundle: &CaBundle, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).context("failed to create CA directory")?;
    fs::write(dir.join("ca.crt"), bundle.cert.pem()).context("failed to write CA cert")?;
    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(dir.join("ca.key"))
        .and_then(|mut f| {
            std::io::Write::write_all(&mut f, bundle.key_pair.serialize_pem().as_bytes())
        })
        .context("failed to write CA key")?;
    Ok(())
}

fn load_ca_from(dir: &Path) -> Result<CaBundle> {
    let key_pem = fs::read_to_string(dir.join("ca.key")).context("failed to read CA key")?;
    let key_pair = KeyPair::from_pem(&key_pem).context("failed to parse CA key")?;

    // Recreate the CA Certificate object for signing leaf certs.
    // The params must match the original CA so the Issuer DN in leaf certs
    // matches the Subject DN of the trusted CA certificate.
    let params = ca_params();
    let cert = params.self_signed(&key_pair)?;

    Ok(CaBundle { cert, key_pair })
}

#[cfg(test)]
fn load_or_create_ca_in(dir: &Path) -> Result<CaBundle> {
    if ca_exists_in(dir) {
        load_ca_from(dir)
    } else {
        let bundle = generate_ca()?;
        save_ca_to(&bundle, dir)?;
        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::MetadataExt;

    use tempfile::TempDir;

    use super::*;
    use crate::tls::cert::generate_leaf_cert;

    #[test]
    fn generates_valid_ca() {
        let bundle = generate_ca().unwrap();
        assert!(bundle.cert.pem().contains("BEGIN CERTIFICATE"));
        assert!(
            bundle
                .key_pair
                .serialize_pem()
                .contains("BEGIN PRIVATE KEY")
        );
    }

    #[test]
    fn save_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let bundle = generate_ca().unwrap();
        save_ca_to(&bundle, dir.path()).unwrap();

        let loaded = load_ca_from(dir.path()).unwrap();
        let (cert, _key) = generate_leaf_cert("round-trip.test", &loaded).unwrap();
        assert!(cert.pem().contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn ca_exists_neither_file() {
        let dir = TempDir::new().unwrap();
        assert!(!ca_exists_in(dir.path()));
    }

    #[test]
    fn ca_exists_cert_only() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("ca.crt"), "cert-data").unwrap();
        assert!(!ca_exists_in(dir.path()));
    }

    #[test]
    fn ca_exists_key_only() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("ca.key"), "key-data").unwrap();
        assert!(!ca_exists_in(dir.path()));
    }

    #[test]
    fn ca_exists_both_files() {
        let dir = TempDir::new().unwrap();
        let bundle = generate_ca().unwrap();
        save_ca_to(&bundle, dir.path()).unwrap();
        assert!(ca_exists_in(dir.path()));
    }

    #[test]
    fn partial_write_recovery() {
        let dir = TempDir::new().unwrap();
        // Simulate partial write: only cert file exists
        let bundle = generate_ca().unwrap();
        fs::write(dir.path().join("ca.crt"), bundle.cert.pem()).unwrap();
        assert!(!ca_exists_in(dir.path()));

        let recovered = load_or_create_ca_in(dir.path()).unwrap();
        assert!(ca_exists_in(dir.path()));
        let (cert, _key) = generate_leaf_cert("recovery.test", &recovered).unwrap();
        assert!(cert.pem().contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn load_or_create_first_run() {
        let dir = TempDir::new().unwrap();
        let bundle = load_or_create_ca_in(dir.path()).unwrap();
        assert!(dir.path().join("ca.crt").exists());
        assert!(dir.path().join("ca.key").exists());
        let (cert, _key) = generate_leaf_cert("first-run.test", &bundle).unwrap();
        assert!(cert.pem().contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn load_or_create_subsequent_run() {
        let dir = TempDir::new().unwrap();
        load_or_create_ca_in(dir.path()).unwrap();
        let key_bytes = fs::read(dir.path().join("ca.key")).unwrap();

        // Second call should load from disk, not regenerate
        load_or_create_ca_in(dir.path()).unwrap();
        let key_bytes_after = fs::read(dir.path().join("ca.key")).unwrap();
        assert_eq!(key_bytes, key_bytes_after);
    }

    #[test]
    fn save_ca_creates_directory() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("nested").join("sub");
        let bundle = generate_ca().unwrap();
        save_ca_to(&bundle, &nested).unwrap();
        assert!(nested.join("ca.crt").exists());
        assert!(nested.join("ca.key").exists());
    }

    #[test]
    fn key_file_permissions() {
        let dir = TempDir::new().unwrap();
        let bundle = generate_ca().unwrap();
        save_ca_to(&bundle, dir.path()).unwrap();
        let mode = fs::metadata(dir.path().join("ca.key")).unwrap().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
