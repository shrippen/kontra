//! Zip archives for `.gp` (Guitar Pro 7/8), `.mxl` (compressed MusicXML), `.kontra` packages and
//! `.kontrakurs` courses, read and written by the `zip` crate (owner, Runde 14).
//!
//! What this layer adds: every read is bounded by the size the archive declares, and callers
//! refuse a declared size before anything is inflated (deflate bombs, lying headers).

use std::io::{Cursor, Read, Write};

use zip::result::ZipError;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::ImportError;

/// Upper bound for one inflated entry; scores are far smaller.
const MAX_ENTRY: u64 = 256 * 1024 * 1024;
/// Upper bound for a score's XML: the parsed tree takes several times that (deflate bomb).
const MAX_SCORE_XML: usize = 32 * 1024 * 1024;
/// Deflate level for written entries (0–9).
const DEFLATE_LEVEL: i64 = 6;

#[derive(Debug)]
pub(crate) struct Zip<'a> {
    archive: ZipArchive<Cursor<&'a [u8]>>,
    /// Name and declared inflated size of each entry, in archive order.
    entries: Vec<(String, u64)>,
}

impl<'a> Zip<'a> {
    pub(crate) fn open(data: &'a [u8]) -> Result<Zip<'a>, ImportError> {
        let mut archive =
            ZipArchive::new(Cursor::new(data)).map_err(|_| ImportError::UnknownFormat)?;
        let mut entries = Vec::with_capacity(archive.len());
        for index in 0..archive.len() {
            let entry = archive.by_index_raw(index).map_err(archive_error)?;
            let name = entry.name().map_err(archive_error)?.into_owned();
            entries.push((name, entry.size()));
        }
        Ok(Zip { archive, entries })
    }

    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(name, _)| name.as_str())
    }

    /// Inflated size of entry `name` as the archive declares it (0 if missing); reading never
    /// yields more.
    pub(crate) fn size(&self, name: &str) -> usize {
        self.entries
            .iter()
            .find(|(n, _)| n == name)
            .map_or(0, |(_, size)| usize::try_from(*size).unwrap_or(usize::MAX))
    }

    /// Content of the XML score entry `name`, refused above `MAX_SCORE_XML` before inflating.
    pub(crate) fn read_score(&self, name: &str) -> Result<Vec<u8>, ImportError> {
        self.read_text(name, MAX_SCORE_XML)
    }

    /// Content of the text entry `name`, refused above `max` bytes before inflating.
    pub(crate) fn read_text(&self, name: &str, max: usize) -> Result<Vec<u8>, ImportError> {
        let size = self.size(name);
        if size > max {
            return Err(ImportError::TooLarge(size as u64));
        }
        self.read(name)
    }

    /// Content of entry `name`: exactly as long as declared, checksum checked.
    pub(crate) fn read(&self, name: &str) -> Result<Vec<u8>, ImportError> {
        // A clone shares the parsed directory; `by_name` needs it mutable.
        let mut archive = self.archive.clone();
        let entry = archive.by_name(name).map_err(|e| match e {
            ZipError::FileNotFound => ImportError::Malformed(format!("{name} missing in archive")),
            other => archive_error(other),
        })?;
        let size = entry.size();
        if size > MAX_ENTRY {
            return Err(ImportError::TooLarge(size));
        }

        // One byte past the declared size tells a lying header; reading to the end checks the CRC.
        let mut content = Vec::with_capacity(size as usize);
        entry
            .take(size + 1)
            .read_to_end(&mut content)
            .map_err(|e| ImportError::Malformed(format!("zip: {e}")))?;
        if content.len() as u64 != size {
            return Err(ImportError::bad("zip entry size mismatch"));
        }
        Ok(content)
    }
}

/// Writes `entries` (name, content) as a zip archive with deflated entries.
pub(crate) fn write(entries: &[(&str, &[u8])]) -> Result<Vec<u8>, ImportError> {
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(DEFLATE_LEVEL));
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in entries {
        writer.start_file(*name, options).map_err(archive_error)?;
        writer.write_all(content)?;
    }
    Ok(writer.finish().map_err(archive_error)?.into_inner())
}

fn archive_error(error: ZipError) -> ImportError {
    match error {
        ZipError::UnsupportedArchive(what) => ImportError::Unsupported(what.to_string()),
        ZipError::CompressionMethodNotSupported(method) => {
            ImportError::Unsupported(format!("zip compression method {method}"))
        }
        other => ImportError::Malformed(format!("zip: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One stored entry whose headers declare `declared` bytes instead of its true length.
    fn stored(name: &str, data: &[u8], declared: u32) -> Vec<u8> {
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file(name, options).unwrap();
        writer.write_all(data).unwrap();
        let mut out = writer.finish().unwrap().into_inner();

        // Uncompressed size: offset 22 in the local header, 24 in the central entry.
        let true_len = (data.len() as u32).to_le_bytes();
        for (signature, offset) in [(0x0403_4b50u32, 22), (0x0201_4b50, 24)] {
            let at = out
                .windows(4)
                .position(|w| w == signature.to_le_bytes())
                .unwrap()
                + offset;
            assert_eq!(out[at..at + 4], true_len);
            out[at..at + 4].copy_from_slice(&declared.to_le_bytes());
        }
        out
    }

    /// Review #131: a stored entry declaring fewer bytes than it holds slipped past every size
    /// limit; reading never yields more than the declared size now.
    #[test]
    fn an_entry_yields_no_more_than_it_declares() {
        let honest = stored("a.md", b"hello", 5);
        assert_eq!(Zip::open(&honest).unwrap().read("a.md").unwrap(), b"hello");

        let lying = stored("a.md", b"hello", 0);
        assert!(Zip::open(&lying).and_then(|z| z.read("a.md")).is_err());
    }

    #[test]
    fn written_archive_reads_back() {
        let big = vec![7u8; 100_000];
        let data = write(&[("manifest.json", b"{}"), ("audio/ü.wav", &big)]).unwrap();
        let zip = Zip::open(&data).unwrap();
        assert_eq!(
            zip.names().collect::<Vec<_>>(),
            ["manifest.json", "audio/ü.wav"]
        );
        assert_eq!(zip.read("manifest.json").unwrap(), b"{}");
        assert_eq!(zip.read("audio/ü.wav").unwrap(), big);
        assert!(data.len() < 10_000, "deflated");
    }

    #[test]
    fn oversized_score_is_refused() {
        // 33 MB of spaces deflate to a few kB: a bomb for the XML parser.
        let bomb = vec![b' '; MAX_SCORE_XML + 1];
        let data = write(&[("score.mscx", &bomb)]).unwrap();
        let zip = Zip::open(&data).unwrap();
        assert!(matches!(
            zip.read_score("score.mscx"),
            Err(ImportError::TooLarge(_))
        ));
    }

    #[test]
    fn rejects_non_zip() {
        assert!(Zip::open(b"not a zip at all, sorry, no").is_err());
        assert!(Zip::open(&[]).is_err());
    }
}
