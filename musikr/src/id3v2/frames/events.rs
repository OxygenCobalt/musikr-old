use crate::id3v2::frames::time::TimestampFormat;
use crate::id3v2::frames::{Frame, FrameConfig, FrameHeader};
use crate::id3v2::{ParseError, ParseResult, TagHeader, Token};
use crate::raw;
use std::fmt::{self, Display, Formatter};

pub struct EventTimingCodesFrame {
    header: FrameHeader,
    time_format: TimestampFormat,
    events: Vec<Event>,
}

impl EventTimingCodesFrame {
    pub fn new() -> Self {
        Self::with_flags(FrameConfig::default())
    }

    pub fn with_flags(flags: FrameConfig) -> Self {
        Self::with_header(FrameHeader::with_flags("ETCO", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        EventTimingCodesFrame {
            header,
            time_format: TimestampFormat::default(),
            events: Vec::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> ParseResult<Self> {
        if data.is_empty() {
            // Cannot be empty
            return Err(ParseError::NotEnoughData);
        }

        let time_format = TimestampFormat::new(data[0]);
        let mut events: Vec<Event> = Vec::new();
        let mut pos = 1;

        while pos + 4 < data.len() {
            let event_type = EventType::new(data[pos]);
            pos += 1;

            let time = raw::to_u32(&data[pos..pos + 4]);
            pos += 4;

            events.push(Event { event_type, time });
        }

        Ok(EventTimingCodesFrame {
            header,
            time_format,
            events,
        })
    }

    pub fn time_format(&self) -> TimestampFormat {
        self.time_format
    }

    pub fn events(&self) -> &Vec<Event> {
        &self.events
    }

    pub fn time_format_mut(&mut self) -> &mut TimestampFormat {
        &mut self.time_format
    }

    pub fn events_mut(&mut self) -> &mut Vec<Event> {
        &mut self.events
    }
}

impl Frame for EventTimingCodesFrame {
    fn key(&self) -> String {
        self.id().clone()
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
        let mut result = vec![self.time_format as u8];

        for event in &self.events {
            result.push(event.event_type as u8);
            result.extend(raw::from_u32(event.time));
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
        Self::with_flags(FrameConfig::default())
    }
}

pub struct Event {
    pub event_type: EventType,
    pub time: u32,
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
    }
}

impl Default for EventType {
    fn default() -> Self {
        EventType::Padding
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let frame = EventTimingCodesFrame::parse(FrameHeader::new("ETCO"), ETCO_DATA).unwrap();
        let events = frame.events();

        assert_eq!(frame.time_format(), TimestampFormat::MpegFrames);

        assert_eq!(events[0].event_type, EventType::IntroStart);
        assert_eq!(events[0].time, 14);
        assert_eq!(events[1].event_type, EventType::IntroEnd);
        assert_eq!(events[1].time, 1234);
        assert_eq!(events[2].event_type, EventType::MainPartStart);
        assert_eq!(events[2].time, 161616);
        assert_eq!(events[3].event_type, EventType::MainPartEnd);
        assert_eq!(events[3].time, 999_999);
    }

    #[test]
    fn render_etco() {
        let mut frame = EventTimingCodesFrame::new();
        *frame.time_format_mut() = TimestampFormat::MpegFrames;
        *frame.events_mut() = vec![
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
        assert_eq!(frame.render(&TagHeader::with_version(4)), ETCO_DATA);
    }
}
