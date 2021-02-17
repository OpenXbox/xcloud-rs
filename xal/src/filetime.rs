use chrono::prelude::*;
use std::convert::TryInto;

/// January 1, 1970 as MS file time
const EPOCH_AS_FILETIME: i64 = 116444736000000000;
const HUNDREDS_OF_NANOSECONDS: i64 = 10000000;

pub trait FileTime<Utc> {
    /// Get DateTime<Utc> that describes FILETIME epoch
    fn filetime_epoch() -> Self;

    /// Creates DateTime<Utc> from FILETIME
    fn from_filetime(filetime: i64) -> Self;

    /// Converts datetime to FILETIME
    fn to_filetime(&self) -> i64;
}

impl FileTime<Utc> for DateTime<Utc> {
    /// Return FILETIME epoch as DateTime<Utc>
    fn filetime_epoch() -> Self {
        let rel_to_linux_epoch = EPOCH_AS_FILETIME;

        let secs: i64 = rel_to_linux_epoch / HUNDREDS_OF_NANOSECONDS;
        let nsecs: i64 = rel_to_linux_epoch % HUNDREDS_OF_NANOSECONDS;

        Utc.timestamp(secs, nsecs.try_into().unwrap())
    }

    /// Example
    /// ```
    /// let dt = Utc.ymd(2009, 7, 25).and_hms_nano(23, 0, 0, 1000);
    /// let ft = DateTime::<Utc>::from_filetime(128930364000001000);
    /// assert_eq!(dt, ft);
    /// ```
    fn from_filetime(filetime: i64) -> DateTime<Utc> {
        let rel_to_linux_epoch = filetime - EPOCH_AS_FILETIME;

        let secs: i64 = rel_to_linux_epoch / HUNDREDS_OF_NANOSECONDS;
        let nsecs: i64 = rel_to_linux_epoch % HUNDREDS_OF_NANOSECONDS;

        Utc.timestamp(secs, nsecs.try_into().unwrap())
    }

    /// Example
    /// ```
    /// let dt = Utc.ymd(2009, 7, 25).and_hms_nano(23, 0, 0, 1000);
    /// assert_eq!(dt.to_filetime(), 128930364000001000);
    /// ```
    fn to_filetime(&self) -> i64 {
        let nsecs = EPOCH_AS_FILETIME + (self.timestamp() * HUNDREDS_OF_NANOSECONDS);
        let remainder: i64 = self.timestamp_subsec_nanos().try_into().unwrap();

        nsecs + remainder
    }
}

#[cfg(test)]
mod test {
    use super::FileTime;
    use chrono::prelude::*;

    #[test]
    fn to_filetime() {
        let dt = Utc.ymd(2009, 7, 25).and_hms_nano(23, 0, 0, 1000);
        assert_eq!(dt.to_filetime(), 128930364000001000);
    }

    #[test]
    fn from_filetime() {
        let dt = Utc.ymd(2009, 7, 25).and_hms_nano(23, 0, 0, 1000);
        let ft = DateTime::<Utc>::from_filetime(128930364000001000);
        assert_eq!(dt, ft);
    }
}
