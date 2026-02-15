use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};
use rustls::crypto::ring as crypto_ring;
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
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

    fn build_certified_key(&self, domain: &str) -> Result<CertifiedKey> {
        let (leaf_cert, leaf_key) = generate_leaf_cert(domain, &self.ca)?;

        let cert_chain = vec![leaf_cert.der().clone(), self.ca.cert.der().clone()];

        let key_der = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(leaf_key.serialize_der()));

        let signing_key = crypto_ring::sign::any_supported_type(&key_der)
            .context("failed to create signing key")?;

        Ok(CertifiedKey::new(cert_chain, signing_key))
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
        let certified_key = Arc::new(self.build_certified_key(domain).ok()?);

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
