//! Minimal zip reader for `.gp` (Guitar Pro 7/8) and `.mxl` (compressed MusicXML):
//! central directory, stored and deflated entries, CRC check. No zip64, no encryption.
//!
//! ```text
//! [local header + data]... │ central directory entries │ end of central directory (EOCD)
//! ```

use crate::error::ImportError;

const EOCD_SIG: u32 = 0x0605_4b50;
const CENTRAL_SIG: u32 = 0x0201_4b50;
const LOCAL_SIG: u32 = 0x0403_4b50;
const EOCD_LEN: usize = 22;
const CENTRAL_LEN: usize = 46;
const LOCAL_LEN: usize = 30;
const MAX_COMMENT: usize = 0xffff;
const STORED: u16 = 0;
const DEFLATED: u16 = 8;
/// Upper bound for one inflated entry; scores are far smaller.
const MAX_ENTRY: usize = 256 * 1024 * 1024;

#[derive(Debug)]
struct Entry {
    name: String,
    method: u16,
    crc: u32,
    packed: usize,
    size: usize,
    local: usize,
}

#[derive(Debug)]
pub(crate) struct Zip<'a> {
    data: &'a [u8],
    entries: Vec<Entry>,
}

impl<'a> Zip<'a> {
    pub(crate) fn open(data: &'a [u8]) -> Result<Zip<'a>, ImportError> {
        let eocd = eocd(data).ok_or(ImportError::UnknownFormat)?;
        let count = usize::from(u16_at(data, eocd + 10).ok_or(ImportError::Truncated(eocd))?);
        let mut at = u32_at(data, eocd + 16).ok_or(ImportError::Truncated(eocd))? as usize;

        let mut entries = Vec::new();
        for _ in 0..count {
            let (entry, next) = central(data, at)?;
            entries.push(entry);
            at = next;
        }
        Ok(Zip { data, entries })
    }

    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.name.as_str())
    }

    /// Content of entry `name`.
    pub(crate) fn read(&self, name: &str) -> Result<Vec<u8>, ImportError> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| ImportError::Malformed(format!("{name} missing in archive")))?;
        let data = self.data;
        let truncated = ImportError::Truncated(entry.local);
        if u32_at(data, entry.local) != Some(LOCAL_SIG) {
            return Err(ImportError::bad("bad local zip header"));
        }
        let name_len = u16_at(data, entry.local + 26).ok_or(truncated)?;
        let extra_len =
            u16_at(data, entry.local + 28).ok_or(ImportError::Truncated(entry.local))?;
        let start = entry.local + LOCAL_LEN + usize::from(name_len) + usize::from(extra_len);
        let packed = data
            .get(start..start.saturating_add(entry.packed))
            .ok_or(ImportError::Truncated(start))?;

        let content = match entry.method {
            STORED => packed.to_vec(),
            DEFLATED => inflate(packed, entry.size)?,
            other => {
                return Err(ImportError::Unsupported(format!(
                    "zip compression method {other}"
                )))
            }
        };
        if crc32(&content) != entry.crc {
            return Err(ImportError::bad("zip entry checksum mismatch"));
        }
        Ok(content)
    }
}

fn inflate(packed: &[u8], size: usize) -> Result<Vec<u8>, ImportError> {
    let limit = size.min(MAX_ENTRY);
    miniz_oxide::inflate::decompress_to_vec_with_limit(packed, limit)
        .map_err(|e| ImportError::Malformed(format!("deflate: {e}")))
}

/// Position of the end-of-central-directory record, searched backwards past a comment.
fn eocd(data: &[u8]) -> Option<usize> {
    let last = data.len().checked_sub(EOCD_LEN)?;
    let first = last.saturating_sub(MAX_COMMENT);
    (first..=last)
        .rev()
        .find(|at| u32_at(data, *at) == Some(EOCD_SIG))
}

/// Parses one central directory entry; returns it and the next entry's position.
fn central(data: &[u8], at: usize) -> Result<(Entry, usize), ImportError> {
    if u32_at(data, at) != Some(CENTRAL_SIG) {
        return Err(ImportError::bad("bad zip central directory"));
    }
    let field = |offset: usize| u32_at(data, at + offset).ok_or(ImportError::Truncated(at));
    let short = |offset: usize| u16_at(data, at + offset).ok_or(ImportError::Truncated(at));
    let name_len = usize::from(short(28)?);
    let extra_len = usize::from(short(30)?);
    let comment_len = usize::from(short(32)?);
    let name_at = at + CENTRAL_LEN;
    let name = data
        .get(name_at..name_at + name_len)
        .ok_or(ImportError::Truncated(name_at))?;
    let entry = Entry {
        name: String::from_utf8_lossy(name).into_owned(),
        method: short(10)?,
        crc: field(16)?,
        packed: field(20)? as usize,
        size: field(24)? as usize,
        local: field(42)? as usize,
    };
    Ok((entry, name_at + name_len + extra_len + comment_len))
}

fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    let bytes = data.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// CRC-32 (IEEE, reflected, polynomial 0xEDB88320) as used by zip.
fn crc32(data: &[u8]) -> u32 {
    const POLY: u32 = 0xedb8_8320;
    let mut crc = u32::MAX;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (POLY & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_of_check_string() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn rejects_non_zip() {
        assert!(Zip::open(b"not a zip at all, sorry, no").is_err());
        assert!(Zip::open(&[]).is_err());
    }
}
