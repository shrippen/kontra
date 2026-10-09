//! Guitar Pro 6 container (`.gpx`): an optionally compressed sector file system.
//!
//! ```text
//! "BCFZ" │ u32 uncompressed size │ bit stream ──decompress──→ "BCFS" │ sectors of 0x1000 bytes
//! sector 0: unused; then file entries: type 2, name at +0x04, size at +0x8c,
//! sector indices (u32, 0-terminated) at +0x94 holding the file data.
//! ```
//!
//! The bit stream (read most significant bit first) is a sequence of chunks:
//! `0` + 2-bit count (LSB first) + count literal bytes, or
//! `1` + 4-bit word size n + n-bit offset + n-bit length (both LSB first): copy from the output.

use crate::error::ImportError;

const BCFZ: &[u8; 4] = b"BCFZ";
const BCFS: &[u8; 4] = b"BCFS";
const SECTOR: usize = 0x1000;
const ENTRY_FILE: u32 = 2;
const NAME_AT: usize = 0x04;
const NAME_LEN: usize = 127;
const SIZE_AT: usize = 0x8c;
const SECTORS_AT: usize = 0x94;
/// Upper bound for the decompressed size; real scores are a few MB.
const MAX_SIZE: usize = 256 * 1024 * 1024;

/// Returns the content of `name` (e.g. `score.gpif`) from a `.gpx` file.
pub(crate) fn file(data: &[u8], name: &str) -> Result<Vec<u8>, ImportError> {
    let plain;
    let system = match data.get(..4) {
        Some(m) if m == BCFZ => {
            plain = decompress(&data[4..])?;
            plain.as_slice()
        }
        Some(m) if m == BCFS => data,
        _ => return Err(ImportError::UnknownFormat),
    };
    if system.get(..4) != Some(BCFS.as_slice()) {
        return Err(ImportError::bad("container lacks BCFS header"));
    }
    find(&system[4..], name)?.ok_or_else(|| ImportError::Malformed(format!("{name} missing")))
}

/// Decompresses a BCFZ stream (after the magic): u32 size, then chunks until the size is reached.
pub(crate) fn decompress(data: &[u8]) -> Result<Vec<u8>, ImportError> {
    let size_bytes: [u8; 4] = data
        .get(..4)
        .and_then(|b| b.try_into().ok())
        .ok_or(ImportError::Truncated(4))?;
    let size = usize::try_from(u32::from_le_bytes(size_bytes)).unwrap_or(usize::MAX);
    if size > MAX_SIZE {
        return Err(ImportError::Malformed(format!("container size {size}")));
    }
    let mut bits = Bits::new(&data[4..]);
    let mut out: Vec<u8> = Vec::new();

    // A stream that ends early leaves the output short; the caller notices missing files.
    while out.len() < size {
        let Some(flag) = bits.read(1) else { break };
        if flag == 1 {
            let Some(word) = bits.read(4) else { break };
            let (Some(offset), Some(len)) = (bits.reversed(word), bits.reversed(word)) else {
                break;
            };
            copy_back(&mut out, offset as usize, len as usize)?;
            continue;
        }
        let Some(count) = bits.reversed(2) else { break };
        for _ in 0..count {
            let Some(byte) = bits.read(8) else { break };
            out.push(byte as u8);
        }
    }
    out.truncate(size);
    Ok(out)
}

/// Appends `len` bytes starting `offset` bytes before the end (at most `offset` bytes).
fn copy_back(out: &mut Vec<u8>, offset: usize, len: usize) -> Result<(), ImportError> {
    let start = out
        .len()
        .checked_sub(offset)
        .filter(|_| offset > 0)
        .ok_or_else(|| ImportError::bad("back reference before start"))?;
    let len = len.min(offset);
    out.extend_from_within(start..start + len);
    Ok(())
}

/// Bit reader, most significant bit of each byte first.
#[derive(Debug)]
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Bits<'a> {
        Bits { data, pos: 0 }
    }

    fn bit(&mut self) -> Option<u32> {
        let byte = *self.data.get(self.pos / 8)?;
        let bit = (byte >> (7 - self.pos % 8)) & 1;
        self.pos += 1;
        Some(u32::from(bit))
    }

    /// `count` bits, first bit most significant.
    fn read(&mut self, count: u32) -> Option<u32> {
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | self.bit()?;
        }
        Some(value)
    }

    /// `count` bits, first bit least significant.
    fn reversed(&mut self, count: u32) -> Option<u32> {
        let mut value = 0;
        for i in 0..count {
            value |= self.bit()? << i;
        }
        Some(value)
    }
}

/// Walks the file entries of an uncompressed file system (after "BCFS").
fn find(system: &[u8], wanted: &str) -> Result<Option<Vec<u8>>, ImportError> {
    let mut offset = SECTOR;
    while offset + 4 <= system.len() {
        if int(system, offset) != Some(ENTRY_FILE) {
            offset += SECTOR;
            continue;
        }
        let name = name(system, offset + NAME_AT);
        let size = int(system, offset + SIZE_AT).unwrap_or(0) as usize;

        // Collect the file's sectors; the next entry follows the last one.
        let mut content = Vec::new();
        let mut slot = offset + SECTORS_AT;
        while let Some(sector) = int(system, slot).filter(|s| *s != 0) {
            let start = (sector as usize).saturating_mul(SECTOR);
            let block = system
                .get(start..start.saturating_add(SECTOR).min(system.len()))
                .ok_or_else(|| ImportError::bad("sector out of range"))?;
            if name == wanted && content.len() < size {
                content.extend_from_slice(block);
            }
            offset = offset.max(start);
            slot += 4;
        }
        if name == wanted {
            content.truncate(size);
            return Ok(Some(content));
        }
        offset += SECTOR;
    }
    Ok(None)
}

fn int(data: &[u8], at: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(at..at.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

/// Zero-terminated name in a fixed field.
fn name(data: &[u8], at: usize) -> String {
    let field = data.get(at..(at + NAME_LEN).min(data.len())).unwrap_or(&[]);
    let end = field.iter().position(|b| *b == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bit writer for hand-built streams.
    struct Writer {
        bytes: Vec<u8>,
        used: usize,
    }

    impl Writer {
        fn bits(&mut self, value: u32, count: u32, reversed: bool) {
            for i in 0..count {
                let shift = if reversed { i } else { count - 1 - i };
                let bit = (value >> shift) & 1;
                if self.used.is_multiple_of(8) {
                    self.bytes.push(0);
                }
                if bit == 1 {
                    let last = self.bytes.len() - 1;
                    self.bytes[last] |= 1 << (7 - self.used % 8);
                }
                self.used += 1;
            }
        }
    }

    #[test]
    fn decompresses_literals_and_back_references() {
        let mut w = Writer {
            bytes: Vec::new(),
            used: 0,
        };
        // Literal "abc".
        w.bits(0, 1, false);
        w.bits(3, 2, true);
        for b in b"abc" {
            w.bits(u32::from(*b), 8, false);
        }
        // Copy 3 bytes from 3 back: "abc" again; word size 2.
        w.bits(1, 1, false);
        w.bits(2, 4, false);
        w.bits(3, 2, true);
        w.bits(3, 2, true);
        // Copy with length > offset is cut to the offset: 1 byte "c".
        w.bits(1, 1, false);
        w.bits(3, 4, false);
        w.bits(1, 3, true);
        w.bits(5, 3, true);

        let mut data = 7u32.to_le_bytes().to_vec();
        data.extend_from_slice(&w.bytes);
        assert_eq!(decompress(&data).unwrap(), b"abcabcc");
    }

    #[test]
    fn rejects_bad_back_reference() {
        // Flag 1, word 1, offset 1 with empty output.
        let data = [9, 0, 0, 0, 0b1000_1110, 0];
        assert!(decompress(&data).is_err());
    }

    #[test]
    fn truncated_stream_is_short_not_panicking() {
        let data = [100, 0, 0, 0, 0b0110_0001];
        assert!(decompress(&data).unwrap().len() < 100);
    }
}
