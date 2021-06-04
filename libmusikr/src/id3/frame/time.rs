use std::fmt::{self, Display, Formatter};

byte_enum! {
    pub enum TimestampFormat {
        Other = 0x00,
        MpegFrames = 0x01,
        Millis = 0x02,
    }
}

impl TimestampFormat {
    pub fn make_timestamp(&self, time: u32) -> Timestamp {
        match self {
            TimestampFormat::Millis => Timestamp::Millis(time),
            TimestampFormat::MpegFrames => Timestamp::MpegFrames(time),
            TimestampFormat::Other => Timestamp::Other(time),
        }
    }
}

impl Default for TimestampFormat {
    fn default() -> Self {
        TimestampFormat::Other
    }
}

pub enum Timestamp {
    Other(u32),
    MpegFrames(u32),
    Millis(u32),
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Timestamp::Millis(time) => write![f, "{}s", time],
            Timestamp::MpegFrames(time) => write![f, "Frame {}", time],
            Timestamp::Other(time) => write![f, "{}", time],
        }
    }
}
