//! HTTPS (plain http only to this machine, for local LLMs) for the few web requests Kontra makes: GET for the Songsterr search (LH-BIB-11) and for
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
    /// For LLM calls: also plain http (checked per URL by `llm_agent`), no redirects, so a
    /// key header never follows one elsewhere.
    llm: ureq::Agent,
    /// The same without proxy, for this machine: a proxy would carry the call elsewhere.
    llm_direct: ureq::Agent,
}

impl Web {
    pub fn new() -> Web {
        Web::build(ureq::Proxy::try_from_env())
    }

    /// `proxy`: the environment's (`HTTPS_PROXY` and kin); tests pass their own.
    fn build(proxy: Option<ureq::Proxy>) -> Web {
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
                .proxy(proxy.clone())
                .tls_config(tls.clone())
                .build()
                .into()
        };
        let llm = |proxy| {
            ureq::Agent::config_builder()
                .timeout_global(Some(LLM_TIMEOUT))
                .user_agent(USER_AGENT)
                .max_redirects(0)
                .proxy(proxy)
                .tls_config(tls.clone())
                .build()
                .into()
        };
        Web {
            agent: agent(TIMEOUT),
            downloads: agent(DOWNLOAD_TIMEOUT),
            llm: llm(proxy.clone()),
            llm_direct: llm(None),
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
        let mut request = self.llm_agent(url)?.get(url);
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
            .llm_agent(url)?
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

/// Whether `host` (as in a URL: IPv6 in brackets) is this machine: `localhost`, 127.0.0.0/8, `::1`.
fn is_loopback(host: &str) -> bool {
    if host == "localhost" {
        return true;
    }
    host.trim_start_matches('[')
        .trim_end_matches(']')
        .parse::<std::net::IpAddr>()
        .is_ok_and(|ip| ip.is_loopback())
}

impl Web {
    /// The agent for an LLM call to `url`, checked before any connection is made. Plain http
    /// only to this machine (Ollama, LM Studio); any other host stays https, so a key never goes
    /// in clear text. This machine is reached direct, never through a proxy.
    fn llm_agent(&self, url: &str) -> Result<&ureq::Agent, HttpError> {
        let uri: ureq::http::Uri = url
            .parse()
            .map_err(|_| HttpError::Failed(format!("bad URL: {url}")))?;
        let local = uri.host().is_some_and(is_loopback);
        match (uri.scheme_str(), local) {
            (Some("https" | "http"), true) => Ok(&self.llm_direct),
            (Some("https"), false) => Ok(&self.llm),
            _ => Err(HttpError::Failed(format!(
                "plain http is only allowed to this machine: {url}"
            ))),
        }
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

    /// Never plain http for Songsterr and package downloads, also not after a redirect from https
    /// (a package could be swapped).
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

    /// Whether `request` holds the headers and as many body bytes as `content-length` says.
    fn request_complete(request: &[u8]) -> bool {
        let text = String::from_utf8_lossy(request);
        let Some(end) = text.find("\r\n\r\n") else {
            return false;
        };
        let length = text[..end]
            .lines()
            .find_map(|l| {
                let (key, value) = l.split_once(':')?;
                key.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        request.len() >= end + 4 + length
    }

    /// One canned answer on a loopback port; returns the base URL.
    fn serve_once() -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            // The whole request, body too: closing with unread bytes resets the connection on
            // Windows (os error 10053) before the client reads the answer.
            let mut request = Vec::new();
            let mut buf = [0u8; 4096];
            while !request_complete(&request) {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buf[..n]),
                }
            }
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
        });
        url
    }

    /// A local provider (Ollama, LM Studio) speaks plain http on loopback.
    #[test]
    fn llm_calls_reach_a_loopback_server_over_http() {
        let web = Web::new();
        assert_eq!(web.get_with(&serve_once(), &[]).unwrap(), b"ok");
        assert_eq!(
            web.post_json(&serve_once(), &[], b"{}", TIMEOUT).unwrap(),
            b"ok"
        );
    }

    /// A proxy from the environment would carry a loopback call, key and all, in clear text to
    /// another machine (closing review of #320): loopback calls go direct.
    #[test]
    fn loopback_calls_bypass_the_proxy() {
        let proxy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_url = format!("http://{}", proxy.local_addr().unwrap());
        proxy.set_nonblocking(true).unwrap();
        let web = Web::build(Some(ureq::Proxy::new(&proxy_url).unwrap()));

        let got = web.post_json(
            &serve_once(),
            &[("authorization", "Bearer k")],
            b"{}",
            TIMEOUT,
        );

        assert!(proxy.accept().is_err(), "the proxy saw the call");
        assert_eq!(got.unwrap(), b"ok");
    }

    /// A key must never travel in clear text to another machine: no connection is made.
    #[test]
    fn llm_calls_refuse_plain_http_to_remote_hosts() {
        let web = Web::new();
        for url in [
            "http://192.0.2.1/v1",
            "http://example.org/v1",
            "http://localhost.example.org/",
        ] {
            let get = web.get_with(url, &[("authorization", "Bearer k")]);
            assert!(
                matches!(&get, Err(HttpError::Failed(m)) if m.contains("plain http")),
                "{url}: {get:?}"
            );
            let post = web.post_json(url, &[], b"{}", TIMEOUT);
            assert!(
                matches!(&post, Err(HttpError::Failed(m)) if m.contains("plain http")),
                "{url}: {post:?}"
            );
        }
    }

    #[test]
    fn loopback_hosts_are_recognised() {
        for host in ["localhost", "127.0.0.1", "127.5.4.3", "[::1]", "::1"] {
            assert!(is_loopback(host), "{host}");
        }
        for host in [
            "192.0.2.1",
            "example.org",
            "localhost.example.org",
            "0.0.0.0",
            "[::2]",
        ] {
            assert!(!is_loopback(host), "{host}");
        }
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
