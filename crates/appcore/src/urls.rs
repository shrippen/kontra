//! File URLs, as QML and the media player want them, to and from paths on each system
//! (LH-NF-17).
//!
//! ```text
//! /home/lea b/a#1.webm     ◄─►  file:///home/lea%20b/a%231.webm
//! C:\Users\Noah\a.webm     ◄─►  file:///C:/Users/Noah/a.webm
//! ```

use std::path::{Path, PathBuf};

const FILE_URL: &str = "file://";

/// How a system writes its paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Paths {
    /// `/home/lea/a.webm`
    Unix,
    /// `C:\Users\Lea\a.webm`
    Windows,
}

impl Paths {
    const HERE: Paths = if cfg!(windows) {
        Paths::Windows
    } else {
        Paths::Unix
    };
}

/// The file URL of `path`: every byte but unreserved ones and `/` percent-encoded, so a name
/// with `#` or `%` ("teil#1.webm") is not read as a fragment or an escape. A Windows path
/// keeps its drive and turns its backslashes:
/// `C:\Users\Lea B\a.webm` → `file:///C:/Users/Lea%20B/a.webm`.
fn file_url_in(path: &str, paths: Paths) -> String {
    let plain: &[u8] = match paths {
        Paths::Unix => b"/-._~",
        Paths::Windows => b"/-._~:",
    };
    let mut out = String::from(FILE_URL);
    // A drive has no leading slash; the URL's path needs one.
    if paths == Paths::Windows && !path.starts_with(['/', '\\']) {
        out.push('/');
    }
    for byte in path.bytes() {
        let byte = match (paths, byte) {
            (Paths::Windows, b'\\') => b'/',
            _ => byte,
        };
        if byte.is_ascii_alphanumeric() || plain.contains(&byte) {
            out.push(char::from(byte));
            continue;
        }
        out.push_str(&format!("%{byte:02X}"));
    }
    out
}

/// The path a file URL names; a plain path stays. Percent escapes are decoded
/// (`a%20b` → `a b`). On Windows a drive loses the URL's own slash (`/C:/a` → `C:/a`) and a
/// share keeps its server (`file://server/a` → `//server/a`).
fn local_path_in(url: &str, paths: Paths) -> String {
    let Some(rest) = url.strip_prefix(FILE_URL) else {
        return url.to_string();
    };
    let path = unescaped(rest);
    if paths == Paths::Unix {
        return path;
    }

    let bytes = path.as_bytes();
    let drive = matches!(bytes, [b'/', letter, b':', ..] if letter.is_ascii_alphabetic());
    if drive {
        return path[1..].to_string();
    }
    if !path.starts_with('/') {
        return format!("//{path}");
    }
    path
}

/// `text` with its percent escapes decoded; a `%` without two hex digits stays.
fn unescaped(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escape = (bytes[i] == b'%')
            .then(|| text.get(i + 1..i + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        if let Some(value) = escape {
            out.push(value);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The file URL of `path` on this system.
pub fn file_url(path: &Path) -> String {
    file_url_in(&path.to_string_lossy(), Paths::HERE)
}

/// The path a file URL names on this system; a plain path stays.
pub fn local_path(url: &str) -> PathBuf {
    PathBuf::from(local_path_in(url, Paths::HERE))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LH-NF-17: a Windows path gave "file://C%3A%5CUsers…", which no player opens.
    #[test]
    fn file_urls_on_both_systems() {
        assert_eq!(
            file_url_in("/home/lea b/teil#1.webm", Paths::Unix),
            "file:///home/lea%20b/teil%231.webm"
        );
        assert_eq!(
            file_url_in(r"C:\Users\Lea B\a.webm", Paths::Windows),
            "file:///C:/Users/Lea%20B/a.webm"
        );
        // A backslash is a letter of the name on Unix.
        assert_eq!(file_url_in(r"/a\b.webm", Paths::Unix), "file:///a%5Cb.webm");
    }

    #[test]
    fn a_file_url_escapes_its_path() {
        assert_eq!(
            file_url_in("/c/media/teil#1 ä%.webm", Paths::Unix),
            "file:///c/media/teil%231%20%C3%A4%25.webm"
        );
    }

    /// LH-NF-17: a file dropped on Kontra under Windows comes as "file:///C:/Users/…"; the
    /// path read from it was "/C:/Users/…", which Windows does not open.
    #[test]
    fn paths_from_urls_on_both_systems() {
        assert_eq!(
            local_path_in("file:///home/a%20b/x.gp5", Paths::Unix),
            "/home/a b/x.gp5"
        );
        assert_eq!(
            local_path_in("/plain/path.mid", Paths::Unix),
            "/plain/path.mid"
        );
        assert_eq!(
            local_path_in("file:///C:/Users/Noah%20B/x.gp5", Paths::Windows),
            "C:/Users/Noah B/x.gp5"
        );
        assert_eq!(
            local_path_in(r"C:\Users\Noah\x.gp5", Paths::Windows),
            r"C:\Users\Noah\x.gp5"
        );
        // A share: file://server/folder/x.gp5.
        assert_eq!(
            local_path_in("file://server/tabs/x.gp5", Paths::Windows),
            "//server/tabs/x.gp5"
        );
    }

    #[test]
    fn a_path_survives_the_round_trip() {
        for (path, paths) in [
            ("/home/lea b/ä#1 100%.webm", Paths::Unix),
            ("C:/Users/Noah B/ä#1 100%.webm", Paths::Windows),
        ] {
            assert_eq!(local_path_in(&file_url_in(path, paths), paths), path);
        }
    }
}
