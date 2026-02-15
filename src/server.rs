use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Router;
use axum::body::Body;
use axum::routing::any;
use hyper::Request;
use hyper::body::Incoming;
use rustls::ServerConfig;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tower::Service;
use tracing::{debug, error, info};

use crate::proxy::{self, ProxyState};
use crate::tls::resolver::CertResolver;

pub async fn run_https_server(
    addr: SocketAddr,
    cert_resolver: Arc<CertResolver>,
    proxy_state: ProxyState,
) -> Result<()> {
    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(cert_resolver);

    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let app = Router::new()
        .fallback(any(proxy::handler))
        .with_state(proxy_state);

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind HTTPS server to {addr}"))?;

    info!(%addr, "HTTPS server listening");

    loop {
        let (stream, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(error = %e, "failed to accept TCP connection");
                continue;
            }
        };

        let acceptor = tls_acceptor.clone();
        let app = app.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    debug!(error = %e, %peer_addr, "TLS handshake failed");
                    return;
                }
            };

            let io = hyper_util::rt::TokioIo::new(tls_stream);

            let tower_service = app;
            let hyper_service = hyper::service::service_fn(move |req: Request<Incoming>| {
                let mut svc = tower_service.clone();
                async move { svc.call(req.map(Body::new)).await }
            });

            if let Err(e) =
                hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                    .serve_connection(io, hyper_service)
                    .await
            {
                debug!(error = %e, %peer_addr, "connection error");
            }
        });
    }
}
