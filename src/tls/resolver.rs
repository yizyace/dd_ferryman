use std::collections::HashMap;
use std::fmt;
use std::io::BufReader;
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};
use rustls::crypto::ring as crypto_ring;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use tracing::{debug, info};

use super::ca::CaBundle;
use super::cert::generate_leaf_cert;

pub struct CertResolver {
    ca: CaBundle,
    cache: RwLock<HashMap<String, Arc<CertifiedKey>>>,
}

impl fmt::Debug for CertResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CertResolver")
            .field("cached_domains", &self.cache.read().ok().map(|c| c.len()))
            .finish_non_exhaustive()
    }
}

impl CertResolver {
    pub fn new(ca: CaBundle) -> Self {
        Self {
            ca,
            cache: RwLock::new(HashMap::new()),
        }
    }

    fn resolve_domain(&self, domain: &str) -> Option<Arc<CertifiedKey>> {
        // Check cache first
        {
            let cache = self.cache.read().ok()?;
            if let Some(key) = cache.get(domain) {
                debug!(domain, "serving cached certificate");
                return Some(Arc::clone(key));
            }
        }

        // Generate new cert
        info!(domain, "generating new certificate");
        let leaf = generate_leaf_cert(domain, &self.ca).ok()?;
        let certified_key =
            build_certified_key(&leaf.cert_pem, &leaf.key_pem, &self.ca.cert_pem).ok()?;
        let certified_key = Arc::new(certified_key);

        // Cache it
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(domain.to_string(), Arc::clone(&certified_key));
        }

        Some(certified_key)
    }
}

impl ResolvesServerCert for CertResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let domain = client_hello.server_name()?;
        self.resolve_domain(domain)
    }
}

fn build_certified_key(cert_pem: &str, key_pem: &str, ca_pem: &str) -> Result<CertifiedKey> {
    let mut certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut BufReader::new(cert_pem.as_bytes()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("failed to parse leaf certificate PEM")?;

    let ca_certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut BufReader::new(ca_pem.as_bytes()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("failed to parse CA certificate PEM")?;

    certs.extend(ca_certs);

    let key: PrivateKeyDer<'static> =
        rustls_pemfile::private_key(&mut BufReader::new(key_pem.as_bytes()))
            .context("failed to parse private key PEM")?
            .context("no private key found in PEM")?;

    let signing_key =
        crypto_ring::sign::any_supported_type(&key).context("failed to create signing key")?;

    Ok(CertifiedKey::new(certs, signing_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tls::ca;

    #[test]
    fn resolves_domain_cert() {
        let ca_bundle = ca::generate_ca().unwrap();
        let resolver = CertResolver::new(ca_bundle);
        let key = resolver.resolve_domain("hello.test");
        assert!(key.is_some());
    }

    #[test]
    fn caches_domain_cert() {
        let ca_bundle = ca::generate_ca().unwrap();
        let resolver = CertResolver::new(ca_bundle);

        // First call generates
        resolver.resolve_domain("cached.test");
        // Second call should use cache
        let key = resolver.resolve_domain("cached.test");
        assert!(key.is_some());
        assert_eq!(resolver.cache.read().unwrap().len(), 1);
    }
}
