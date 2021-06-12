use crate::id3::frame::time::{Timestamp, TimestampFormat};
use crate::id3::frame::{FrameHeader, Id3Frame};
use crate::raw;
use std::fmt::{self, Display, Formatter};

pub struct EventTimingCodesFrame {
    header: FrameHeader,
    time_format: TimestampFormat,
    events: Vec<Event>,
}

impl EventTimingCodesFrame {
    pub fn new(header: FrameHeader) -> Self {
        EventTimingCodesFrame {
            header,
            time_format: TimestampFormat::default(),
            events: Vec::new(),
        }
    }
}

impl Id3Frame for EventTimingCodesFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn key(&self) -> String {
        self.id().clone()
    }

    fn parse(&mut self, data: &[u8]) -> Result<(), ()> {
        if data.is_empty() {
            return Err(()); // Not enough data
        }

        self.time_format = TimestampFormat::new(data[0]);
        let mut pos = 1;

        while pos + 4 < data.len() {
            let event_type = EventType::new(data[pos]);
            pos += 1;

            let timestamp = self.time_format.make_timestamp(raw::to_u32(&data[pos..]));
            pos += 4;

            self.events.push(Event {
                event_type,
                timestamp,
            });
        }

        Ok(())
    }
}

impl Display for EventTimingCodesFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for event in &self.events {
            write![f, "\n{}", event]?;
        }

        Ok(())
    }
}

pub struct Event {
    event_type: EventType,
    timestamp: Timestamp,
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.event_type]
    }
}

byte_enum! {
    pub enum EventType {
        Padding = 0x00,
        EndOfInitialSilence = 0x01,
        IntroStart = 0x02,
        MainPartStart = 0x03,
        OutroStart = 0x04,
        OutroEnd = 0x05,
        VerseStart = 0x06,
        RefrainStart = 0x07,
        InterludeStart = 0x08,
        ThemeStart = 0x09,
        VariationStart = 0x0A,
        KeyChange = 0x0B,
        TimeChange = 0x0C,
        MomentaryUnwantedNoise = 0x0D,
        SustainedNoise = 0x0E,
        SustainedNoiseEnd = 0x0F,
        IntroEnd = 0x10,
        MainPartEnd = 0x11,
        VerseEnd = 0x12,
        RefrainEnd = 0x13,
        ThemeEnd = 0x14,
        Profanity = 0x15,
        ProfanityEnd = 0x16,
        Sync0 = 0xE0,
        Sync1 = 0xE1,
        Sync2 = 0xE2,
        Sync3 = 0xE3,
        Sync4 = 0xE4,
        Sync5 = 0xE5,
        Sync6 = 0xE6,
        Sync7 = 0xE7,
        Sync8 = 0xE8,
        Sync9 = 0xE9,
        SyncA = 0xEA,
        SyncB = 0xEB,
        SyncC = 0xEC,
        SyncD = 0xED,
        SyncE = 0xEE,
        SyncF = 0xEF,
        AudioEnd = 0xFD,
        AudioFileEnd = 0xFE,
    }
}

impl Display for EventType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{:?}", self]
    }
}

impl Default for EventType {
    fn default() -> Self {
        EventType::Padding
    }
}
