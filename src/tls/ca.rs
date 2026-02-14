use std::fs;
use std::os::unix::fs::OpenOptionsExt;

use anyhow::{Context, Result};
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair, KeyUsagePurpose};

use crate::paths;

pub struct CaBundle {
    pub cert_pem: String,
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
    let cert = paths::ca_cert_path()?;
    let key = paths::ca_key_path()?;
    Ok(cert.exists() && key.exists())
}

pub fn generate_ca() -> Result<CaBundle> {
    let params = ca_params();
    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;
    let cert_pem = cert.pem();

    Ok(CaBundle {
        cert_pem,
        cert,
        key_pair,
    })
}

pub fn load_or_create_ca() -> Result<CaBundle> {
    if ca_exists()? {
        load_ca()
    } else {
        let bundle = generate_ca()?;
        save_ca(&bundle)?;
        Ok(bundle)
    }
}

fn save_ca(bundle: &CaBundle) -> Result<()> {
    paths::ensure_dirs()?;
    fs::write(paths::ca_cert_path()?, &bundle.cert_pem).context("failed to write CA cert")?;
    let key_path = paths::ca_key_path()?;
    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&key_path)
        .and_then(|mut f| {
            std::io::Write::write_all(&mut f, bundle.key_pair.serialize_pem().as_bytes())
        })
        .context("failed to write CA key")?;
    Ok(())
}

fn load_ca() -> Result<CaBundle> {
    let cert_pem = fs::read_to_string(paths::ca_cert_path()?).context("failed to read CA cert")?;
    let key_pem = fs::read_to_string(paths::ca_key_path()?).context("failed to read CA key")?;

    let key_pair = KeyPair::from_pem(&key_pem).context("failed to parse CA key")?;

    // Recreate the CA Certificate object for signing leaf certs.
    // The params must match the original CA so the Issuer DN in leaf certs
    // matches the Subject DN of the trusted CA certificate.
    let params = ca_params();
    let cert = params.self_signed(&key_pair)?;

    Ok(CaBundle {
        cert_pem,
        cert,
        key_pair,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_ca() {
        let bundle = generate_ca().unwrap();
        assert!(bundle.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(
            bundle
                .key_pair
                .serialize_pem()
                .contains("BEGIN PRIVATE KEY")
        );
    }
}
