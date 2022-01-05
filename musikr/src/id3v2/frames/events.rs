//! Frames for timed media events.

use crate::core::io::BufStream;
use crate::id3v2::frames::{Frame, FrameId};
use crate::id3v2::{ParseResult, TagHeader};
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::fmt::{self, Display, Formatter};

#[derive(Default, Debug, Clone)]
pub struct EventTimingCodesFrame {
    pub format: TimestampFormat,
    pub events: Vec<Event>,
}

impl EventTimingCodesFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let format = TimestampFormat::parse(stream.read_u8()?);
        let mut events: Vec<Event> = Vec::new();

        while !stream.is_empty() {
            let event_type = EventType::parse(stream.read_u8()?);
            let time = stream.read_be_u32()?;

            events.push(Event { event_type, time });
        }

        Ok(Self { format, events })
    }
}

impl Frame for EventTimingCodesFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"ETCO")
    }

    fn key(&self) -> String {
        String::from("ETCO")
    }

    fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        let mut result = vec![self.format as u8];

        // Technically events should be sorted by their time, but nobody seems to care about this.
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
    #[derive(Ord, PartialOrd)]
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

#[derive(Clone, Debug)]
pub struct SyncedTempoCodesFrame {
    format: TimestampFormat,
    tempos: Vec<Tempo>
}

impl SyncedTempoCodesFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let format = TimestampFormat::parse(stream.read_u8()?);
        let mut tempos = Vec::new();

        while !stream.is_empty() {
            tempos.push(Tempo::parse(stream)?)
        }

        Ok(Self { format, tempos })
    }
}

impl Frame for SyncedTempoCodesFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"SYTC")
    }

    fn key(&self) -> String {
        String::from("SYTC")
    }

    fn is_empty(&self) -> bool {
        self.tempos.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        let mut data = vec![self.format as u8];

        for tempo in &self.tempos {
            data.extend(tempo.render())
        }

        data
    }
}

impl Display for SyncedTempoCodesFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for (i, tempo) in self.tempos.iter().enumerate() {
            write![f, "{}", tempo.bpm.0]?;

            if i < self.tempos.len() - 1 {
                write![f, ", "]?;
            }
        }

        Ok(())        
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Tempo {
    pub bpm: Bpm,
    pub time: u32
}

impl Tempo {
    fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        // ID3v2 decides to encode the BPM as a one or two byte sequence.
        // So a u8 or a u16, right? WRONG! The bytes are actually meant
        // to be added together, resulting in a maximum of 512 BPM, a
        // value that is not the cap for any built-in type! Yay! 
        let mut bpm = Bpm(stream.read_u8()?.into());

        if bpm.0 == u8::MAX.into() {
            bpm.0 += Into::<u16>::into(stream.read_u8()?);
        }

        // Assume the timestamp is a u32, like it always is.
        let time = stream.read_be_u32()?;

        Ok(Self { bpm, time })
    }

    fn render(&self) -> Vec<u8> {
        let bpm = u16::min(self.bpm.0, u8::MAX as u16 * 2);

        let mut data: Vec<u8> = match bpm.checked_sub(u8::MAX.into()) {
            Some(remainder) => vec![0xFF, remainder as u8],
            None => vec![bpm as u8]
        };

        data.extend(self.time.to_be_bytes());
        data
    }
}

/// The representation of a BPM interval in [`SyncedTempoCodesFrame`](SyncedTempoCodesFrame).
///
/// While this value is encoded as a u16, the field is actually only
/// capped at 512 BPM. Any value above that will be rounded to down
/// to 512. 
///
/// Certain BPM values also have a special meaning in the ID3v2
/// specification. A BPM of 0 indiciates a "beat-free" interval,
/// while a BPM of 1 indicates a single beat followed by a beat-free
/// interval. Musikr does not represent that, as it's assumed to be
/// implicit by the nature of this type.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Bpm(pub u16);

byte_enum! {
    /// A representation of an ID3v2 timestamp format
    ///
    /// The timestamp format represents the units for any timestamps
    /// in an ID3v2 frame. For the best compatibility with programs,
    /// [`Millis`](TimestampFormat::Millis) should be used.
    pub enum TimestampFormat {
        /// No unit was specified.
        Other = 0x00,
        /// Timestamps are in MPEG Frames.
        MpegFrames = 0x01,
        /// Timestamps are in milliseconds.
        Millis = 0x02,
    };
    TimestampFormat::Other
}

impl Default for TimestampFormat {
    fn default() -> Self {
        TimestampFormat::Millis
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ETCO_DATA: &[u8] = b"ETCO\x00\x00\x00\x15\x00\x00\
                               \x02\
                               \x02\
                               \x00\x00\x00\x0E\
                               \x10\
                               \x00\x00\x04\xD2\
                               \x03\
                               \x00\x02\x77\x50\
                               \x11\
                               \x00\x0F\x42\x3F";

    const SYTC_DATA: &[u8] = b"SYTC\x00\x00\x00\x22\x00\x00\
                               \x02\
                               \x00\
                               \x00\x00\x00\x0E\
                               \x01\
                               \x00\x00\x04\xD2\
                               \xFF\x00\
                               \x00\x02\x77\x50\
                               \xFF\xFF\
                               \x00\x0F\x42\x3F\
                               \x16\
                               \x16\x16\x16\x16\
                               \xFF\xA0\
                               \x00\x00\x00\x00";

    #[test]
    fn parse_etco() {
        make_frame!(EventTimingCodesFrame, ETCO_DATA, frame);

        assert_eq!(frame.format, TimestampFormat::Millis);
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
        let frame = EventTimingCodesFrame {
            format: TimestampFormat::Millis,
            events: vec![
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
            ],
        };

        assert_render!(frame, ETCO_DATA);
    }

    #[test]
    fn parse_sylt() {
        make_frame!(SyncedTempoCodesFrame, SYTC_DATA, frame);

        assert_eq!(frame.format, TimestampFormat::Millis);
        assert_eq!(frame.tempos[0], Tempo { bpm: Bpm(0), time: 14 });
        assert_eq!(frame.tempos[1], Tempo { bpm: Bpm(1), time: 1234 });
        assert_eq!(frame.tempos[2], Tempo { bpm: Bpm(255), time: 161616 });
        assert_eq!(frame.tempos[3], Tempo { bpm: Bpm(510), time: 999_999 });
        assert_eq!(frame.tempos[4], Tempo { bpm: Bpm(22), time: 0x16161616 });
        assert_eq!(frame.tempos[5], Tempo { bpm: Bpm(415), time: 0 });
    }

    #[test]
    fn render_sylt() {
        let frame = SyncedTempoCodesFrame {
            format: TimestampFormat::Millis,
            tempos: vec![
                Tempo { bpm: Bpm(0), time: 14 },
                Tempo { bpm: Bpm(1), time: 1234 },
                Tempo { bpm: Bpm(255), time: 161616 },
                Tempo { bpm: Bpm(510), time: 999_999 },
                Tempo { bpm: Bpm(22), time: 0x16161616 },
                Tempo { bpm: Bpm(415), time: 0 }
            ]
        };

        assert_render!(frame, SYTC_DATA);        
    }

    #[test]
    fn parse_timestamp_format() {
        assert_eq!(TimestampFormat::parse(0), TimestampFormat::Other);
        assert_eq!(TimestampFormat::parse(1), TimestampFormat::MpegFrames);
        assert_eq!(TimestampFormat::parse(2), TimestampFormat::Millis);

        for i in 3..u8::MAX {
            assert_eq!(TimestampFormat::parse(i), TimestampFormat::Other)
        }
    }

    #[test]
    fn render_timestamp_format() {
        assert_eq!(TimestampFormat::Other as u8, 0);
        assert_eq!(TimestampFormat::MpegFrames as u8, 1);
        assert_eq!(TimestampFormat::Millis as u8, 2);
    }
}
