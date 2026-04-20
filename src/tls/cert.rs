use anyhow::{Context, Result};
use rcgen::{CertificateParams, KeyPair};
use time::{Duration, OffsetDateTime};

use super::ca::CaBundle;

// Apple HT211025: iOS trust evaluation rejects TLS leaves whose total validity
// exceeds 398 days. 397 days is the conventional safety margin.
const LEAF_BACKDATE_DAYS: i64 = 1;
const LEAF_FORWARD_DAYS: i64 = 396;

pub fn generate_leaf_cert(domain: &str, ca: &CaBundle) -> Result<(rcgen::Certificate, KeyPair)> {
    let mut params = CertificateParams::new(vec![domain.to_string(), format!("*.{domain}")])
        .context("failed to create leaf cert params")?;

    let now = OffsetDateTime::now_utc();
    params.not_before = now - Duration::days(LEAF_BACKDATE_DAYS);
    params.not_after = now + Duration::days(LEAF_FORWARD_DAYS);

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

    #[test]
    fn leaf_validity_within_ios_trust_policy() {
        let total = LEAF_BACKDATE_DAYS + LEAF_FORWARD_DAYS;
        assert!(
            total <= 397,
            "leaf validity {total} days exceeds iOS 397-day cap"
        );
    }
}
