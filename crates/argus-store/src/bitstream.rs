//! Most-significant-bit-first bit writer and reader, the substrate for the
//! [Gorilla](crate::gorilla) codec.

/// Appends individual bits into a growing byte buffer, MSB first.
#[derive(Debug, Default)]
pub struct BitWriter {
    bytes: Vec<u8>,
    current: u8,
    filled: u8,
}

impl BitWriter {
    pub fn new() -> Self {
        BitWriter::default()
    }

    /// Append a single bit.
    pub fn write_bit(&mut self, bit: bool) {
        self.current = (self.current << 1) | u8::from(bit);
        self.filled += 1;
        if self.filled == 8 {
            self.bytes.push(self.current);
            self.current = 0;
            self.filled = 0;
        }
    }

    /// Append the low `count` bits of `value`, most significant first.
    pub fn write_bits(&mut self, value: u64, count: u8) {
        for shift in (0..count).rev() {
            self.write_bit((value >> shift) & 1 == 1);
        }
    }

    /// Total number of bits written so far.
    pub fn bit_len(&self) -> usize {
        self.bytes.len() * 8 + self.filled as usize
    }

    /// The encoded bytes, with any partial trailing byte zero-padded on the
    /// right. Non-consuming, so encoding can continue afterwards.
    pub fn snapshot(&self) -> Vec<u8> {
        let mut out = self.bytes.clone();
        if self.filled > 0 {
            out.push(self.current << (8 - self.filled));
        }
        out
    }
}

/// Reads bits back out of a byte slice, MSB first.
#[derive(Debug)]
pub struct BitReader<'a> {
    bytes: &'a [u8],
    byte: usize,
    bit: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        BitReader {
            bytes,
            byte: 0,
            bit: 0,
        }
    }

    /// Read a single bit, or `None` once the buffer is exhausted.
    pub fn read_bit(&mut self) -> Option<bool> {
        let byte = *self.bytes.get(self.byte)?;
        let bit = (byte >> (7 - self.bit)) & 1 == 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.byte += 1;
        }
        Some(bit)
    }

    /// Read `count` bits into the low bits of a `u64`, most significant first.
    pub fn read_bits(&mut self, count: u8) -> Option<u64> {
        let mut value = 0u64;
        for _ in 0..count {
            value = (value << 1) | u64::from(self.read_bit()?);
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_mixed_widths() {
        let mut writer = BitWriter::new();
        writer.write_bit(true);
        writer.write_bits(0b101, 3);
        writer.write_bits(0xDEAD_BEEF, 32);
        writer.write_bits(u64::MAX, 64);
        writer.write_bit(false);

        let bytes = writer.snapshot();
        let mut reader = BitReader::new(&bytes);
        assert_eq!(reader.read_bit(), Some(true));
        assert_eq!(reader.read_bits(3), Some(0b101));
        assert_eq!(reader.read_bits(32), Some(0xDEAD_BEEF));
        assert_eq!(reader.read_bits(64), Some(u64::MAX));
        assert_eq!(reader.read_bit(), Some(false));
    }

    #[test]
    fn reports_exhaustion() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b1, 1);
        let bytes = writer.snapshot();
        let mut reader = BitReader::new(&bytes);
        assert_eq!(reader.read_bit(), Some(true));
        // remaining padding bits read as zero until the byte is exhausted
        for _ in 0..7 {
            assert_eq!(reader.read_bit(), Some(false));
        }
        assert_eq!(reader.read_bit(), None);
    }
}
