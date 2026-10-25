//! HTTPS GET for the few web requests Kontra makes (the Songsterr search, LH-BIB-11). Behind a
//! trait, so tests and screenshots use canned answers instead of the network.
//!
//! ```text
//! session ─► dyn Http ─┬─► Web (ureq, rustls, system certificates)
//!                      └─► FileHttp (QA: one file as every answer)
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

/// A slow answer is no answer: the user waits for it.
const TIMEOUT: Duration = Duration::from_secs(8);
/// Search answers are kilobytes; anything near this is not one.
const MAX_BODY: u64 = 2 * 1024 * 1024;
/// A download (a course package) may take a slow line its time.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
/// Names Kontra and where to find it, so the site knows who asks.
const USER_AGENT: &str = concat!(
    "Kontra/",
    env!("CARGO_PKG_VERSION"),
    " (bass trainer; +",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HttpError {
    /// No connection: offline, name not found, refused.
    #[error("no connection: {0}")]
    Offline(String),
    #[error("no answer within {} s", TIMEOUT.as_secs())]
    Timeout,
    #[error("HTTP status {0}")]
    Status(u16),
    #[error("{0}")]
    Failed(String),
}

/// Fetches a URL; one call is one request.
pub trait Http: Send + Sync + std::fmt::Debug {
    fn get(&self, url: &str) -> Result<Vec<u8>, HttpError>;

    /// A file to keep (a course package): up to `max_bytes`, with time for a slow line.
    fn download(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, HttpError> {
        let _ = max_bytes;
        self.get(url)
    }
}

/// The real web.
#[derive(Debug)]
pub struct Web {
    agent: ureq::Agent,
    /// The same, with time for downloads.
    downloads: ureq::Agent,
}

impl Web {
    pub fn new() -> Web {
        use ureq::tls::{Certificate, RootCerts, TlsConfig, TlsProvider};
        // The system's roots (its CA store); unreadable ones are skipped, none means no HTTPS
        // and the search says it is offline.
        let roots: Vec<Certificate<'static>> = rustls_native_certs::load_native_certs()
            .certs
            .iter()
            .map(|der| Certificate::from_der(der.as_ref()).to_owned())
            .collect();
        let tls = TlsConfig::builder()
            .provider(TlsProvider::Rustls)
            .root_certs(RootCerts::new_with_certs(&roots))
            .build();
        let agent = |timeout| {
            ureq::Agent::config_builder()
                .timeout_global(Some(timeout))
                .user_agent(USER_AGENT)
                // Also after a redirect: courses and searches are https only.
                .https_only(true)
                .tls_config(tls.clone())
                .build()
                .into()
        };
        Web {
            agent: agent(TIMEOUT),
            downloads: agent(DOWNLOAD_TIMEOUT),
        }
    }
}

impl Default for Web {
    fn default() -> Web {
        Web::new()
    }
}

impl Http for Web {
    fn get(&self, url: &str) -> Result<Vec<u8>, HttpError> {
        let mut response = self.agent.get(url).call().map_err(error)?;
        response
            .body_mut()
            .with_config()
            .limit(MAX_BODY)
            .read_to_vec()
            .map_err(error)
    }

    fn download(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, HttpError> {
        let mut response = self.downloads.get(url).call().map_err(error)?;
        response
            .body_mut()
            .with_config()
            .limit(max_bytes)
            .read_to_vec()
            .map_err(error)
    }
}

fn error(e: ureq::Error) -> HttpError {
    match e {
        ureq::Error::StatusCode(code) => HttpError::Status(code),
        ureq::Error::Timeout(_) => HttpError::Timeout,
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => {
            HttpError::Offline(e.to_string())
        }
        ureq::Error::Io(io) => HttpError::Offline(io.to_string()),
        other => HttpError::Failed(other.to_string()),
    }
}

/// Answers every request with one file (QA runs, screenshots): never the network.
#[derive(Debug, Clone)]
pub struct FileHttp {
    path: PathBuf,
}

impl FileHttp {
    pub fn new(path: &Path) -> FileHttp {
        FileHttp {
            path: path.to_path_buf(),
        }
    }
}

impl Http for FileHttp {
    fn get(&self, _url: &str) -> Result<Vec<u8>, HttpError> {
        std::fs::read(&self.path).map_err(|e| HttpError::Failed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_names_kontra() {
        assert!(USER_AGENT.starts_with("Kontra/"));
        assert!(USER_AGENT.contains("+https://"));
    }

    #[test]
    fn file_answers_every_url() {
        let path = std::env::temp_dir().join(format!("kontra-http-{}", std::process::id()));
        std::fs::write(&path, b"[]").unwrap();
        let http = FileHttp::new(&path);
        assert_eq!(http.get("https://example.org/a").unwrap(), b"[]");
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(http.get("x"), Err(HttpError::Failed(_))));
    }

    /// Never plain http, also not after a redirect from https (a package could be swapped).
    #[test]
    fn plain_http_is_never_asked() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/kurs", listener.local_addr().unwrap());
        let web = Web::new();
        assert!(web.get(&url).is_err());
        assert!(web.download(&url, 1).is_err());
        assert!(listener.accept().is_err(), "no connection was made");
    }
}
