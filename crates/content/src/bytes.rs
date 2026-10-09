//! Bounds-checked little-endian reader for binary formats. Every read returns
//! `ImportError::Truncated` instead of panicking when the data ends.

use crate::error::ImportError;

#[derive(Debug)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Reader<'a> {
        Reader { data, pos: 0 }
    }

    pub(crate) fn take(&mut self, len: usize) -> Result<&'a [u8], ImportError> {
        let end = self
            .pos
            .checked_add(len)
            .filter(|end| *end <= self.data.len())
            .ok_or(ImportError::Truncated(self.pos))?;
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    pub(crate) fn at_end(&self) -> bool {
        self.pos >= self.data.len()
    }

    pub(crate) fn skip(&mut self, len: usize) -> Result<(), ImportError> {
        self.take(len).map(|_| ())
    }

    pub(crate) fn u8(&mut self) -> Result<u8, ImportError> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn i8(&mut self) -> Result<i8, ImportError> {
        Ok(i8::from_le_bytes([self.u8()?]))
    }

    pub(crate) fn bool(&mut self) -> Result<bool, ImportError> {
        Ok(self.u8()? != 0)
    }

    pub(crate) fn i16(&mut self) -> Result<i16, ImportError> {
        let b = self.take(2)?;
        Ok(i16::from_le_bytes([b[0], b[1]]))
    }

    pub(crate) fn i32(&mut self) -> Result<i32, ImportError> {
        let b = self.take(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// A length or count that must not be negative.
    pub(crate) fn count(&mut self) -> Result<usize, ImportError> {
        let value = self.i32()?;
        usize::try_from(value)
            .map_err(|_| ImportError::Malformed(format!("negative count {value}")))
    }

    /// Byte length followed by a fixed-size field of `size` bytes.
    pub(crate) fn byte_string(&mut self, size: usize) -> Result<String, ImportError> {
        let len = usize::from(self.u8()?);
        let field = self.take(size)?;
        Ok(latin1(&field[..len.min(size)]))
    }

    /// 32-bit length followed by that many bytes.
    pub(crate) fn int_string(&mut self) -> Result<String, ImportError> {
        let len = self.count()?;
        Ok(latin1(self.take(len)?))
    }

    /// 32-bit field size, then a byte length and the field (Guitar Pro's usual string).
    pub(crate) fn int_byte_string(&mut self) -> Result<String, ImportError> {
        let size = self.count()?;
        if size == 0 {
            return Err(ImportError::bad("string field of size 0"));
        }
        self.byte_string(size - 1)
    }
}

/// Guitar Pro stores text in the system code page; Latin-1 keeps the common letters.
fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|b| char::from(*b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_little_endian() {
        let mut r = Reader::new(&[1, 0xff, 0x34, 0x12, 0x78, 0x56, 0x34, 0x12]);
        assert_eq!(r.u8().unwrap(), 1);
        assert_eq!(r.i8().unwrap(), -1);
        assert_eq!(r.i16().unwrap(), 0x1234);
        assert_eq!(r.i32().unwrap(), 0x1234_5678);
        assert!(matches!(r.u8(), Err(ImportError::Truncated(8))));
    }

    #[test]
    fn strings() {
        let mut r = Reader::new(&[4, 0, 0, 0, 3, b'a', b'b', b'c', 2, b'x', b'y', 0]);
        assert_eq!(r.int_byte_string().unwrap(), "abc");
        assert_eq!(r.byte_string(2).unwrap(), "xy");
        assert!(r.byte_string(5).is_err());
    }
}
