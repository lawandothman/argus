//! Nanosecond-precision wall-clock timestamps.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// A wall-clock timestamp in nanoseconds since the Unix epoch.
///
/// Nanoseconds is the native resolution of the OTLP wire format, so storing it
/// directly avoids lossy conversions on ingest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(u64);

impl Timestamp {
    /// The Unix epoch, `1970-01-01T00:00:00Z`.
    pub const EPOCH: Timestamp = Timestamp(0);

    /// The current wall-clock time.
    ///
    /// Clamps to [`Timestamp::EPOCH`] if the system clock is set before the
    /// Unix epoch, so the value is always representable.
    pub fn now() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos() as u64)
            .unwrap_or(0);
        Timestamp(nanos)
    }

    /// Construct from nanoseconds since the Unix epoch.
    pub const fn from_unix_nanos(nanos: u64) -> Self {
        Timestamp(nanos)
    }

    /// Construct from milliseconds since the Unix epoch.
    pub const fn from_unix_millis(millis: u64) -> Self {
        Timestamp(millis.saturating_mul(1_000_000))
    }

    /// Nanoseconds since the Unix epoch.
    pub const fn as_unix_nanos(self) -> u64 {
        self.0
    }

    /// Whole milliseconds since the Unix epoch (truncated).
    pub const fn as_unix_millis(self) -> u64 {
        self.0 / 1_000_000
    }

    /// Seconds since the Unix epoch as a float — convenient for charting.
    pub fn as_unix_secs_f64(self) -> f64 {
        self.0 as f64 / 1_000_000_000.0
    }

    /// Nanoseconds elapsed from `earlier` to `self`, saturating at zero when
    /// `self` precedes `earlier`.
    pub const fn saturating_nanos_since(self, earlier: Timestamp) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn millis_and_nanos_agree() {
        let ts = Timestamp::from_unix_millis(1_500);
        assert_eq!(ts.as_unix_nanos(), 1_500_000_000);
        assert_eq!(ts.as_unix_millis(), 1_500);
    }

    #[test]
    fn difference_saturates_instead_of_underflowing() {
        let earlier = Timestamp::from_unix_nanos(100);
        let later = Timestamp::from_unix_nanos(250);
        assert_eq!(later.saturating_nanos_since(earlier), 150);
        assert_eq!(earlier.saturating_nanos_since(later), 0);
    }

    #[test]
    fn now_is_after_the_epoch() {
        assert!(Timestamp::now() > Timestamp::EPOCH);
    }
}
