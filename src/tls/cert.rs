use anyhow::{Context, Result};
use rcgen::{CertificateParams, KeyPair};
use time::{Duration, OffsetDateTime};

use super::ca::CaBundle;

pub fn generate_leaf_cert(domain: &str, ca: &CaBundle) -> Result<(rcgen::Certificate, KeyPair)> {
    let mut params = CertificateParams::new(vec![domain.to_string(), format!("*.{domain}")])
        .context("failed to create leaf cert params")?;

    let now = OffsetDateTime::now_utc();
    params.not_before = now - Duration::days(1);
    params.not_after = now + Duration::days(825);

    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, domain);

    let leaf_key = KeyPair::generate()?;
    let cert = params
        .signed_by(&leaf_key, &ca.cert, &ca.key_pair)
        .context("failed to sign leaf certificate")?;

    Ok((cert, leaf_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tls::ca;

    #[test]
    fn generates_valid_leaf_cert() {
        let ca_bundle = ca::generate_ca().unwrap();
        let (cert, key) = generate_leaf_cert("hello.test", &ca_bundle).unwrap();
        assert!(cert.pem().contains("BEGIN CERTIFICATE"));
        assert!(key.serialize_pem().contains("BEGIN PRIVATE KEY"));
    }
}
