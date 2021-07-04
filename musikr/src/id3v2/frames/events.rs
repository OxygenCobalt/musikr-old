use crate::core::io::BufStream;
use crate::id3v2::frames::time::TimestampFormat;
use crate::id3v2::frames::{Frame, FrameHeader, FrameId, Token};
use crate::id3v2::{ParseResult, TagHeader};
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct EventTimingCodesFrame {
    header: FrameHeader,
    pub format: TimestampFormat,
    pub events: Vec<Event>,
}

impl EventTimingCodesFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let format = TimestampFormat::parse(stream.read_u8()?);
        let mut events: Vec<Event> = Vec::new();

        while !stream.is_empty() {
            let event_type = EventType::parse(stream.read_u8()?);
            let time = stream.read_u32()?;

            events.push(Event { event_type, time });
        }

        Ok(Self {
            header,
            format,
            events,
        })
    }
}

impl Frame for EventTimingCodesFrame {
    fn key(&self) -> String {
        String::from("ETCO")
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        let mut result = vec![self.format as u8];

        for event in &self.events {
            result.push(event.event_type as u8);
            result.extend(event.time.to_be_bytes());
        }

        result
    }
}

impl Display for EventTimingCodesFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for (i, event) in self.events.iter().enumerate() {
            write![f, "{}", event]?;

            if i < self.events.len() - 1 {
                write![f, ", "]?;
            }
        }

        Ok(())
    }
}

impl Default for EventTimingCodesFrame {
    fn default() -> Self {
        Self {
            header: FrameHeader::new(FrameId::new(b"ETCO")),
            format: TimestampFormat::default(),
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Default)]
pub struct Event {
    pub event_type: EventType,
    pub time: u32,
}

impl Ord for Event {
    /// Compares the time first, then event type.
    fn cmp(&self, other: &Self) -> Ordering {
        match self.time.cmp(&other.time) {
            Ordering::Equal => self.event_type.cmp(&other.event_type),
            ord => ord,
        }
    }
}

impl PartialOrd<Self> for Event {
    /// Compares the time first, then event type.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{:?}", self.event_type]
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
    };
    EventType::Padding
}

impl Default for EventType {
    fn default() -> Self {
        EventType::Padding
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::tag::Version;

    const ETCO_DATA: &[u8] = b"\x01\
                                \x02\
                                \x00\x00\x00\x0E\
                                \x10\
                                \x00\x00\x04\xD2\
                                \x03\
                                \x00\x02\x77\x50\
                                \x11\
                                \x00\x0F\x42\x3F";

    #[test]
    fn parse_etco() {
        let frame = EventTimingCodesFrame::parse(
            FrameHeader::new(FrameId::new(b"ETCO")),
            &mut BufStream::new(ETCO_DATA),
        )
        .unwrap();

        assert_eq!(frame.format, TimestampFormat::MpegFrames);

        assert_eq!(frame.events[0].event_type, EventType::IntroStart);
        assert_eq!(frame.events[0].time, 14);
        assert_eq!(frame.events[1].event_type, EventType::IntroEnd);
        assert_eq!(frame.events[1].time, 1234);
        assert_eq!(frame.events[2].event_type, EventType::MainPartStart);
        assert_eq!(frame.events[2].time, 161616);
        assert_eq!(frame.events[3].event_type, EventType::MainPartEnd);
        assert_eq!(frame.events[3].time, 999_999);
    }

    #[test]
    fn render_etco() {
        let mut frame = EventTimingCodesFrame::new();
        frame.format = TimestampFormat::MpegFrames;
        frame.events = vec![
            Event {
                event_type: EventType::IntroStart,
                time: 14,
            },
            Event {
                event_type: EventType::IntroEnd,
                time: 1234,
            },
            Event {
                event_type: EventType::MainPartStart,
                time: 161616,
            },
            Event {
                event_type: EventType::MainPartEnd,
                time: 999_999,
            },
        ];

        assert!(!frame.is_empty());
        assert_eq!(
            frame.render(&TagHeader::with_version(Version::V24)),
            ETCO_DATA
        );
    }
}
