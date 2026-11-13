//! HTTPS for the few web requests Kontra makes: GET for the Songsterr search (LH-BIB-11) and for
//! an LLM's model list, POST for an LLM's answer (LH-BIB-16, the "Stück wünschen" wish). Behind a
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
/// An LLM can take minutes to write a whole piece plan.
const LLM_TIMEOUT: Duration = Duration::from_secs(180);
/// A piece plan is a few kilobytes of JSON; this is already generous.
const MAX_LLM_BODY: u64 = 4 * 1024 * 1024;
/// An error body (a provider's explanation) kept in the error, cut to this many bytes.
const MAX_ERROR_BODY: usize = 2000;
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
    /// An error status with the body the provider sent back (its own explanation), cut short.
    #[error("HTTP status {0}: {1}")]
    StatusBody(u16, String),
    #[error("{0}")]
    Failed(String),
    /// This `Http` implementation does not do this kind of request (the default, for GET-only
    /// fakes); a real provider call never sees it.
    #[error("not supported")]
    NotSupported,
}

/// Fetches a URL; one call is one request.
pub trait Http: Send + Sync + std::fmt::Debug {
    fn get(&self, url: &str) -> Result<Vec<u8>, HttpError>;

    /// A file to keep (a course package): up to `max_bytes`, with time for a slow line.
    fn download(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, HttpError> {
        let _ = max_bytes;
        self.get(url)
    }

    /// A GET with extra headers (an LLM provider's model list, sent with its auth header).
    fn get_with(&self, url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>, HttpError> {
        let _ = headers;
        self.get(url)
    }

    /// A POST of a JSON body, with extra headers (an LLM provider's chat/messages call). The
    /// default errors, so a fake that only implements `get` still compiles.
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, HttpError> {
        let _ = (url, headers, body, timeout);
        Err(HttpError::NotSupported)
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

    fn get_with(&self, url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>, HttpError> {
        let mut request = self.agent.get(url);
        for (key, value) in headers {
            request = request.header(*key, *value);
        }
        // Errors stay a normal response (not Err), so a failing status keeps its body: the
        // provider's explanation of what went wrong.
        let response = request
            .config()
            .timeout_global(Some(LLM_TIMEOUT))
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(error)?;
        body_or_status(response, MAX_LLM_BODY)
    }

    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, HttpError> {
        let mut request = self
            .agent
            .post(url)
            .header("content-type", "application/json");
        for (key, value) in headers {
            request = request.header(*key, *value);
        }
        let response = request
            .config()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .build()
            .send(body)
            .map_err(error)?;
        body_or_status(response, MAX_LLM_BODY)
    }
}

/// Reads a response's body; past a 4xx/5xx status, the body is the provider's error text and
/// goes into the error instead, cut to `MAX_ERROR_BODY`.
fn body_or_status(
    mut response: ureq::http::Response<ureq::Body>,
    limit: u64,
) -> Result<Vec<u8>, HttpError> {
    let status = response.status().as_u16();
    let read = response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(error)?;
    if !(200..300).contains(&status) {
        return Err(HttpError::StatusBody(status, error_text(&read)));
    }
    Ok(read)
}

/// The error body as text, trimmed and cut to `MAX_ERROR_BODY` (a provider's explanation can run
/// long; the UI shows one line).
fn error_text(body: &[u8]) -> String {
    let mut text = String::from_utf8_lossy(body).trim().to_string();
    text.truncate(MAX_ERROR_BODY);
    text
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
        assert!(web.get_with(&url, &[]).is_err());
        assert!(web.post_json(&url, &[], b"{}", TIMEOUT).is_err());
        assert!(listener.accept().is_err(), "no connection was made");
    }

    /// An LLM's error body, e.g. `{"error":"bad model"}`, is what the UI shows; long ones are cut.
    #[test]
    fn error_text_trims_and_cuts() {
        assert_eq!(
            error_text(b"  {\"error\":\"bad model\"}\n"),
            "{\"error\":\"bad model\"}"
        );
        let long = "x".repeat(MAX_ERROR_BODY + 500);
        assert_eq!(error_text(long.as_bytes()).len(), MAX_ERROR_BODY);
    }

    /// The default `post_json`/`get_with` (for a fake `Http` that only implements `get`) never
    /// silently falls back to a GET: an LLM call on such a fake is a clear "not supported".
    #[test]
    fn default_post_json_is_not_supported() {
        #[derive(Debug)]
        struct GetOnly;
        impl Http for GetOnly {
            fn get(&self, _url: &str) -> Result<Vec<u8>, HttpError> {
                Ok(b"ok".to_vec())
            }
        }
        let http = GetOnly;
        assert_eq!(
            http.post_json("https://example.org", &[], b"{}", TIMEOUT),
            Err(HttpError::NotSupported)
        );
    }
}
