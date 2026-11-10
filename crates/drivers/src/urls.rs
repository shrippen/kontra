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
fn file_url_in(path: &[u8], paths: Paths) -> String {
    let plain: &[u8] = match paths {
        Paths::Unix => b"/-._~",
        Paths::Windows => b"/-._~:",
    };
    let mut out = String::from(FILE_URL);
    // A drive has no leading slash; the URL's path needs one.
    if paths == Paths::Windows && !path.starts_with(b"/") && !path.starts_with(b"\\") {
        out.push('/');
    }
    for &byte in path {
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

/// The path a file URL names, as its bytes; a plain path stays. Percent escapes are decoded
/// (`a%20b` → `a b`). On Windows a drive loses the URL's own slash (`/C:/a` → `C:/a`) and a
/// share keeps its server (`file://server/a` → `//server/a`).
fn local_path_bytes(url: &str, paths: Paths) -> Vec<u8> {
    let Some(rest) = url.strip_prefix(FILE_URL) else {
        return url.as_bytes().to_vec();
    };
    let mut path = unescaped(rest);
    if paths == Paths::Unix {
        return path;
    }

    let drive = matches!(path[..], [b'/', letter, b':', ..] if letter.is_ascii_alphabetic());
    if drive {
        path.remove(0);
        return path;
    }
    if !path.starts_with(b"/") {
        path.splice(0..0, *b"//");
    }
    path
}

/// `local_path_bytes` as text, for a system whose paths are text (and for the tests).
#[cfg(any(test, not(unix)))]
fn local_path_in(url: &str, paths: Paths) -> String {
    String::from_utf8_lossy(&local_path_bytes(url, paths)).into_owned()
}

/// `text` with its percent escapes decoded; a `%` without two hex digits stays.
fn unescaped(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escape = match bytes.get(i..i + 3) {
            Some([b'%', high, low]) => hex_digit(*high).zip(hex_digit(*low)),
            _ => None,
        };
        if let Some((high, low)) = escape {
            out.push(high * HEX + low);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

const HEX: u8 = 16;

/// The value of a hex digit; a sign or another letter is none.
fn hex_digit(byte: u8) -> Option<u8> {
    char::from(byte).to_digit(u32::from(HEX)).map(|d| d as u8)
}

/// The file URL of `path` on this system; a relative path counts from the working folder.
pub fn file_url(path: &Path) -> String {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    #[cfg(unix)]
    {
        // The name's own bytes: a name that is not UTF-8 stays what it is.
        use std::os::unix::ffi::OsStrExt;
        file_url_in(path.as_os_str().as_bytes(), Paths::HERE)
    }
    #[cfg(not(unix))]
    {
        file_url_in(path.to_string_lossy().as_bytes(), Paths::HERE)
    }
}

/// The path a file URL names on this system; a plain path stays.
pub fn local_path(url: &str) -> PathBuf {
    #[cfg(unix)]
    {
        // The name's own bytes, as `file_url` wrote them.
        use std::os::unix::ffi::OsStringExt;
        PathBuf::from(std::ffi::OsString::from_vec(local_path_bytes(
            url,
            Paths::HERE,
        )))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(local_path_in(url, Paths::HERE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LH-NF-17: a Windows path gave "file://C%3A%5CUsers…", which no player opens.
    #[test]
    fn file_urls_on_both_systems() {
        assert_eq!(
            file_url_in(b"/home/lea b/teil#1.webm", Paths::Unix),
            "file:///home/lea%20b/teil%231.webm"
        );
        assert_eq!(
            file_url_in(br"C:\Users\Lea B\a.webm", Paths::Windows),
            "file:///C:/Users/Lea%20B/a.webm"
        );
        // A backslash is a letter of the name on Unix.
        assert_eq!(
            file_url_in(br"/a\b.webm", Paths::Unix),
            "file:///a%5Cb.webm"
        );
    }

    #[test]
    fn a_file_url_escapes_its_path() {
        assert_eq!(
            file_url_in("/c/media/teil#1 ä%.webm".as_bytes(), Paths::Unix),
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

    /// Review #233: a name that is not UTF-8 lost its bytes on the way (U+FFFD).
    #[cfg(unix)]
    #[test]
    fn a_name_keeps_its_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let path = Path::new(std::ffi::OsStr::from_bytes(b"/c/\xFF/a.webm"));
        assert_eq!(file_url(path), "file:///c/%FF/a.webm");
    }

    /// Review #233: a relative path got a slash in front and pointed at the drive's root.
    #[test]
    fn a_relative_path_counts_from_here() {
        let here = std::env::current_dir().unwrap();
        assert_eq!(
            file_url(Path::new("media/a.webm")),
            file_url(&here.join("media/a.webm"))
        );
    }

    /// Review #234: a dropped file whose name is not UTF-8 came back with U+FFFD in its name
    /// and did not open.
    #[cfg(unix)]
    #[test]
    fn a_dropped_name_keeps_its_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let path = local_path("file:///c/%FF/a.webm");
        assert_eq!(path.as_os_str().as_bytes(), b"/c/\xFF/a.webm");
        assert_eq!(local_path(&file_url(&path)), path);
    }

    /// Review #234: "%+1" was read as an escape (a sign is no hex digit).
    #[test]
    fn only_hex_digits_escape() {
        assert_eq!(local_path_in("file:///a%+1b", Paths::Unix), "/a%+1b");
        assert_eq!(local_path_in("file:///a%4", Paths::Unix), "/a%4");
        assert_eq!(local_path_in("file:///a%41", Paths::Unix), "/aA");
    }

    #[test]
    fn a_path_survives_the_round_trip() {
        for (path, paths) in [
            ("/home/lea b/ä#1 100%.webm", Paths::Unix),
            ("C:/Users/Noah B/ä#1 100%.webm", Paths::Windows),
        ] {
            assert_eq!(
                local_path_in(&file_url_in(path.as_bytes(), paths), paths),
                path
            );
        }
    }
}
