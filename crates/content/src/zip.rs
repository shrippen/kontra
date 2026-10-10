//! Minimal zip reader for `.gp` (Guitar Pro 7/8) and `.mxl` (compressed MusicXML):
//! central directory, stored and deflated entries, CRC check. No zip64, no encryption.
//! The writer (`.kontra` packages) stores deflated entries the same way.
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

/// Deflate level for written entries (miniz: 0–10).
const DEFLATE_LEVEL: u8 = 6;
/// Version 2.0: deflate; the same for "made by" and "needed to extract".
const ZIP_VERSION: u16 = 20;
/// Bit 11: names are UTF-8.
const UTF8_NAMES: u16 = 1 << 11;

/// Writes `entries` (name, content) as a zip archive with deflated entries, no zip64: every
/// entry and the archive must stay below 4 GiB, and at most 65535 entries.
pub(crate) fn write(entries: &[(&str, &[u8])]) -> Result<Vec<u8>, ImportError> {
    let too_large = |n: usize| ImportError::TooLarge(n as u64);
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, content) in entries {
        let packed = miniz_oxide::deflate::compress_to_vec(content, DEFLATE_LEVEL);
        let local = u32::try_from(out.len()).map_err(|_| too_large(out.len()))?;
        let size = u32::try_from(content.len()).map_err(|_| too_large(content.len()))?;
        let packed_len = u32::try_from(packed.len()).map_err(|_| too_large(packed.len()))?;
        let name_len =
            u16::try_from(name.len()).map_err(|_| ImportError::bad("zip name too long"))?;
        let crc = crc32(content);

        // Local header: signature, version, flags, method, time, date, crc, sizes, name.
        out.extend(LOCAL_SIG.to_le_bytes());
        for v in [ZIP_VERSION, UTF8_NAMES, DEFLATED, 0, 0] {
            out.extend(v.to_le_bytes());
        }
        for v in [crc, packed_len, size] {
            out.extend(v.to_le_bytes());
        }
        out.extend(name_len.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out.extend(name.as_bytes());
        out.extend(&packed);

        // Central entry: as above plus made-by, comment, disk, attributes and the offset.
        central.extend(CENTRAL_SIG.to_le_bytes());
        for v in [ZIP_VERSION, ZIP_VERSION, UTF8_NAMES, DEFLATED, 0, 0] {
            central.extend(v.to_le_bytes());
        }
        for v in [crc, packed_len, size] {
            central.extend(v.to_le_bytes());
        }
        for v in [name_len, 0, 0, 0, 0] {
            central.extend(v.to_le_bytes());
        }
        central.extend(0u32.to_le_bytes());
        central.extend(local.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let count = u16::try_from(entries.len()).map_err(|_| ImportError::bad("too many entries"))?;
    let central_at = u32::try_from(out.len()).map_err(|_| too_large(out.len()))?;
    let central_len = u32::try_from(central.len()).map_err(|_| too_large(central.len()))?;
    out.extend(central);
    out.extend(EOCD_SIG.to_le_bytes());
    for v in [0, 0, count, count] {
        out.extend(v.to_le_bytes());
    }
    out.extend(central_len.to_le_bytes());
    out.extend(central_at.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    Ok(out)
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
    fn rejects_non_zip() {
        assert!(Zip::open(b"not a zip at all, sorry, no").is_err());
        assert!(Zip::open(&[]).is_err());
    }
}
