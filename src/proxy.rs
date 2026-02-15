use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use http_body_util::BodyExt;
use hyper::Request;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use tracing::{debug, warn};

#[derive(Clone)]
pub struct ProxyState {
    apps_dir: PathBuf,
    client: Client<hyper_util::client::legacy::connect::HttpConnector, Body>,
}

impl ProxyState {
    pub fn new(apps_dir: PathBuf) -> Self {
        let client = Client::builder(TokioExecutor::new()).build_http();
        Self { apps_dir, client }
    }
}

fn extract_app_name(host: &str) -> Option<&str> {
    let host = host.split(':').next().unwrap_or(host);
    host.strip_suffix(".test")
}

fn resolve_port(apps_dir: &Path, app_name: &str) -> Option<u16> {
    let path = apps_dir.join(app_name);
    let contents = std::fs::read_to_string(&path).ok()?;
    contents.trim().parse().ok()
}

#[allow(clippy::missing_panics_doc)]
pub async fn handler(State(state): State<ProxyState>, req: axum::extract::Request) -> Response {
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();

    let Some(app_name) = extract_app_name(&host).map(ToOwned::to_owned) else {
        warn!(%host, "no .test domain in Host header");
        return (StatusCode::NOT_FOUND, "no .test domain in Host header").into_response();
    };

    let Some(port) = resolve_port(&state.apps_dir, &app_name) else {
        warn!(%app_name, "no app registered");
        return (
            StatusCode::NOT_FOUND,
            format!("no app registered for {app_name}.test"),
        )
            .into_response();
    };

    debug!(%app_name, port, "proxying request");

    let (mut parts, body) = req.into_parts();

    let path_and_query = parts.uri.path_and_query().map_or("/", |pq| pq.as_str());

    let upstream_uri: Uri = format!("http://127.0.0.1:{port}{path_and_query}")
        .parse()
        .expect("valid URI from controlled format string");

    parts.uri = upstream_uri;
    parts
        .headers
        .insert("x-forwarded-proto", HeaderValue::from_static("https"));

    let upstream_req = Request::from_parts(parts, body);

    match state.client.request(upstream_req).await {
        Ok(resp) => {
            let (parts, body) = resp.into_parts();
            let body_bytes = body
                .collect()
                .await
                .map_or_else(|_| bytes::Bytes::new(), http_body_util::Collected::to_bytes);
            Response::from_parts(parts, Body::from(body_bytes))
        }
        Err(e) => {
            warn!(%app_name, port, error = %e, "upstream connection failed");
            (StatusCode::BAD_GATEWAY, "upstream connection failed").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_app_name_strips_test_suffix() {
        assert_eq!(extract_app_name("formyx.test"), Some("formyx"));
    }

    #[test]
    fn extract_app_name_strips_port_and_suffix() {
        assert_eq!(extract_app_name("formyx.test:443"), Some("formyx"));
    }

    #[test]
    fn extract_app_name_returns_none_for_non_test() {
        assert_eq!(extract_app_name("example.com"), None);
    }

    #[test]
    fn extract_app_name_handles_nested_subdomain() {
        assert_eq!(extract_app_name("api.formyx.test"), Some("api.formyx"));
    }

    #[test]
    fn resolve_port_reads_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("myapp"), "3000\n").unwrap();
        assert_eq!(resolve_port(dir.path(), "myapp"), Some(3000));
    }

    #[test]
    fn resolve_port_returns_none_for_missing_app() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(resolve_port(dir.path(), "missing"), None);
    }

    #[test]
    fn resolve_port_returns_none_for_invalid_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("badapp"), "not-a-port").unwrap();
        assert_eq!(resolve_port(dir.path(), "badapp"), None);
    }
}
