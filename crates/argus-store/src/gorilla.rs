//! Gorilla time-series compression (Facebook's "Gorilla" paper).
//!
//! Timestamps are encoded as delta-of-delta with variable-width buckets; values
//! are XOR'd against the previous value and only the changed (meaningful) bits
//! are stored. For regularly-spaced series this approaches ~1 bit per timestamp
//! and a handful of bits per value.
//!
//! Timestamps here are milliseconds since the epoch — metrics are stored at
//! millisecond resolution (the OTLP nanosecond input is downsampled on append),
//! which is both standard for a TSDB and what makes delta-of-delta pay off.

use crate::bitstream::{BitReader, BitWriter};

/// An append-only, Gorilla-compressed run of `(timestamp_ms, value)` samples.
#[derive(Debug)]
pub struct GorillaChunk {
    writer: BitWriter,
    count: usize,
    timestamp: u64,
    delta: i64,
    value_bits: u64,
    leading: u32,
    trailing: u32,
    started: bool,
}

impl Default for GorillaChunk {
    fn default() -> Self {
        GorillaChunk {
            writer: BitWriter::new(),
            count: 0,
            timestamp: 0,
            delta: 0,
            value_bits: 0,
            leading: u32::MAX,
            trailing: 0,
            started: false,
        }
    }
}

impl GorillaChunk {
    pub fn new() -> Self {
        GorillaChunk::default()
    }

    /// Number of samples appended.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Encoded size in bytes (including any partial trailing byte).
    pub fn encoded_len(&self) -> usize {
        self.writer.bit_len().div_ceil(8)
    }

    /// Append a sample. Timestamps are expected to be non-decreasing.
    pub fn push(&mut self, timestamp_ms: u64, value: f64) {
        let bits = value.to_bits();

        if !self.started {
            self.writer.write_bits(timestamp_ms, 64);
            self.writer.write_bits(bits, 64);
            self.timestamp = timestamp_ms;
            self.value_bits = bits;
            self.started = true;
            self.count = 1;
            return;
        }

        let delta = timestamp_ms as i64 - self.timestamp as i64;
        self.write_timestamp(delta - self.delta);
        self.timestamp = timestamp_ms;
        self.delta = delta;

        self.write_value(bits);
        self.value_bits = bits;
        self.count += 1;
    }

    /// Decode all samples back out.
    pub fn samples(&self) -> Vec<(u64, f64)> {
        decode(&self.writer.snapshot(), self.count)
    }

    fn write_timestamp(&mut self, dod: i64) {
        // Buckets sized to exact two's-complement ranges to avoid off-by-one.
        if dod == 0 {
            self.writer.write_bit(false);
        } else if (-64..=63).contains(&dod) {
            self.writer.write_bits(0b10, 2);
            self.writer.write_bits(dod as u64, 7);
        } else if (-256..=255).contains(&dod) {
            self.writer.write_bits(0b110, 3);
            self.writer.write_bits(dod as u64, 9);
        } else if (-2048..=2047).contains(&dod) {
            self.writer.write_bits(0b1110, 4);
            self.writer.write_bits(dod as u64, 12);
        } else {
            self.writer.write_bits(0b1111, 4);
            self.writer.write_bits(dod as u64, 64);
        }
    }

    fn write_value(&mut self, bits: u64) {
        let xor = self.value_bits ^ bits;
        if xor == 0 {
            self.writer.write_bit(false);
            return;
        }
        self.writer.write_bit(true);

        let leading = xor.leading_zeros().min(31);
        let trailing = xor.trailing_zeros();

        if self.leading != u32::MAX && leading >= self.leading && trailing >= self.trailing {
            // Reuse the previous window of meaningful bits.
            self.writer.write_bit(false);
            let meaningful = 64 - self.leading - self.trailing;
            self.writer
                .write_bits(xor >> self.trailing, meaningful as u8);
        } else {
            // Open a new window.
            self.writer.write_bit(true);
            self.writer.write_bits(u64::from(leading), 5);
            let meaningful = 64 - leading - trailing;
            self.writer.write_bits(u64::from(meaningful - 1), 6);
            self.writer.write_bits(xor >> trailing, meaningful as u8);
            self.leading = leading;
            self.trailing = trailing;
        }
    }
}

/// Decode `count` samples from a Gorilla byte buffer.
fn decode(bytes: &[u8], count: usize) -> Vec<(u64, f64)> {
    let mut out = Vec::with_capacity(count);
    if count == 0 {
        return out;
    }

    let mut reader = BitReader::new(bytes);
    let mut timestamp = reader.read_bits(64).unwrap_or(0);
    let mut value_bits = reader.read_bits(64).unwrap_or(0);
    out.push((timestamp, f64::from_bits(value_bits)));

    let mut delta: i64 = 0;
    let mut leading = 0u32;
    let mut trailing = 0u32;

    for _ in 1..count {
        let Some(dod) = read_timestamp(&mut reader) else {
            break;
        };
        delta += dod;
        timestamp = (timestamp as i64 + delta) as u64;

        match read_value_control(&mut reader) {
            ValueControl::Same => {}
            ValueControl::ReuseWindow => {
                let meaningful = 64 - leading - trailing;
                let bits = reader.read_bits(meaningful as u8).unwrap_or(0);
                value_bits ^= bits << trailing;
            }
            ValueControl::NewWindow => {
                leading = reader.read_bits(5).unwrap_or(0) as u32;
                let meaningful = reader.read_bits(6).unwrap_or(0) as u32 + 1;
                trailing = 64 - leading - meaningful;
                let bits = reader.read_bits(meaningful as u8).unwrap_or(0);
                value_bits ^= bits << trailing;
            }
            ValueControl::Eof => break,
        }

        out.push((timestamp, f64::from_bits(value_bits)));
    }

    out
}

fn read_timestamp(reader: &mut BitReader) -> Option<i64> {
    if !reader.read_bit()? {
        return Some(0);
    }
    if !reader.read_bit()? {
        return Some(sign_extend(reader.read_bits(7)?, 7));
    }
    if !reader.read_bit()? {
        return Some(sign_extend(reader.read_bits(9)?, 9));
    }
    if !reader.read_bit()? {
        return Some(sign_extend(reader.read_bits(12)?, 12));
    }
    Some(reader.read_bits(64)? as i64)
}

enum ValueControl {
    Same,
    ReuseWindow,
    NewWindow,
    Eof,
}

fn read_value_control(reader: &mut BitReader) -> ValueControl {
    match reader.read_bit() {
        Some(false) => ValueControl::Same,
        Some(true) => match reader.read_bit() {
            Some(false) => ValueControl::ReuseWindow,
            Some(true) => ValueControl::NewWindow,
            None => ValueControl::Eof,
        },
        None => ValueControl::Eof,
    }
}

/// Interpret the low `bits` of `value` as a two's-complement signed integer.
fn sign_extend(value: u64, bits: u8) -> i64 {
    let shift = 64 - bits;
    ((value << shift) as i64) >> shift
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(samples: &[(u64, f64)]) {
        let mut chunk = GorillaChunk::new();
        for &(timestamp, value) in samples {
            chunk.push(timestamp, value);
        }
        assert_eq!(chunk.count(), samples.len());
        let decoded = chunk.samples();
        assert_eq!(decoded.len(), samples.len());
        for (got, want) in decoded.iter().zip(samples) {
            assert_eq!(got.0, want.0, "timestamp mismatch");
            assert_eq!(got.1.to_bits(), want.1.to_bits(), "value mismatch");
        }
    }

    #[test]
    fn empty_and_single() {
        round_trip(&[]);
        round_trip(&[(1_700_000_000_000, 42.0)]);
    }

    #[test]
    fn regular_timestamps_constant_value() {
        let samples: Vec<_> = (0..1_000).map(|i| (1_000 + i * 1_000, 7.5)).collect();
        round_trip(&samples);
    }

    #[test]
    fn irregular_timestamps_random_walk() {
        let mut value = 0.0_f64;
        let mut timestamp = 1_700_000_000_000u64;
        let mut state = 0x1234_5678u64;
        let mut samples = Vec::new();
        for _ in 0..2_000 {
            // cheap xorshift for deterministic pseudo-randomness
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            timestamp += 20 + state % 130;
            value += (state % 1000) as f64 / 500.0 - 1.0;
            samples.push((timestamp, value));
        }
        round_trip(&samples);
    }

    #[test]
    fn monotonic_counter_and_special_floats() {
        let mut samples: Vec<_> = (0..500).map(|i| (1_000 + i * 250, i as f64)).collect();
        samples.push((samples.last().unwrap().0 + 250, f64::MAX));
        samples.push((samples.last().unwrap().0 + 250, -0.0));
        samples.push((samples.last().unwrap().0 + 250, 0.0));
        round_trip(&samples);
    }

    #[test]
    fn actually_compresses() {
        let samples: Vec<_> = (0..1_000).map(|i| (1_000 + i * 1_000, 100.0)).collect();
        let mut chunk = GorillaChunk::new();
        for &(t, v) in &samples {
            chunk.push(t, v);
        }
        // 1000 samples of regular timestamps + constant value must be far
        // smaller than the 16 KB a raw (i64, f64) layout would need.
        assert!(
            chunk.encoded_len() < 1_000,
            "expected strong compression, got {}",
            chunk.encoded_len()
        );
    }
}
