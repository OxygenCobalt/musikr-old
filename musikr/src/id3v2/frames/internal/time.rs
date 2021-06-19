byte_enum! {
    pub enum TimestampFormat {
        Other = 0x00,
        MpegFrames = 0x01,
        Millis = 0x02,
    }
}

impl Default for TimestampFormat {
    fn default() -> Self {
        TimestampFormat::Millis
    }
}
