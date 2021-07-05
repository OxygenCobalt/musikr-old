use crate::core::io::BufStream;
use crate::id3v2::frames::{self, Frame, FrameId};
use crate::id3v2::{FrameMap, ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct ChapterFrame {
    pub element_id: String,
    pub time: ChapterTime,
    pub frames: FrameMap,
}

impl ChapterFrame {
    pub(crate) fn parse(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let element_id = string::read_terminated(Encoding::Latin1, stream);

        let time = ChapterTime {
            start_time: stream.read_u32()?,
            end_time: stream.read_u32()?,
            start_offset: stream.read_u32()?,
            end_offset: stream.read_u32()?,
        };

        // Recursively call frames::new to get any embedded frames.
        let mut frames = FrameMap::new();

        while let Ok(frame) = frames::new(tag_header, stream) {
            frames.add(frame);
        }

        Ok(Self {
            element_id,
            time,
            frames,
        })
    }
}

impl Frame for ChapterFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"CHAP")
    }

    fn key(&self) -> String {
        format!["CHAP:{}", self.element_id]
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        Vec::new()
    }
}

impl Display for ChapterFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![
            f,
            "{} [Start: {}, End: {}]",
            self.element_id, self.time.start_time, self.time.end_time
        ]?;

        if !self.frames.is_empty() {
            write![f, " Sub-Frames:"]?;

            for frame in self.frames.values() {
                write![f, " {}", frame.id()]?;
            }
        }

        Ok(())
    }
}

impl Default for ChapterFrame {
    fn default() -> Self {
        Self {
            element_id: String::new(),
            time: ChapterTime::default(),
            frames: FrameMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChapterTime {
    pub start_time: u32,
    pub end_time: u32,
    pub start_offset: u32,
    pub end_offset: u32,
}

impl Default for ChapterTime {
    fn default() -> Self {
        ChapterTime {
            start_time: 0,
            end_time: 0,
            start_offset: u32::MAX,
            end_offset: u32::MAX,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableOfContentsFrame {
    pub element_id: String,
    pub flags: TocFlags,
    pub elements: Vec<String>,
    pub frames: FrameMap,
}

impl TableOfContentsFrame {
    pub(crate) fn parse(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let element_id = string::read_terminated(Encoding::Latin1, stream);

        let flags = stream.read_u8()?;
        let flags = TocFlags {
            top_level: flags & 0x2 != 0,
            ordered: flags & 0x1 != 0,
        };

        let mut elements: Vec<String> = Vec::new();
        let entry_count = stream.read_u8()?;

        for _ in 0..entry_count {
            if stream.is_empty() {
                // The entry count may be inaccurate, so we also ensure that we
                // don't overread the data.
                break;
            }

            elements.push(string::read_terminated(Encoding::Latin1, stream));
        }

        let mut frames = FrameMap::new();

        // Second loop, this time to get any embedded frames.
        while let Ok(frame) = frames::new(tag_header, stream) {
            frames.add(frame);
        }

        Ok(Self {
            element_id,
            flags,
            elements,
            frames,
        })
    }
}

impl Frame for TableOfContentsFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"CTOC")
    }

    fn key(&self) -> String {
        format!["CTOC:{}", self.element_id]
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        Vec::new()
    }
}

impl Display for TableOfContentsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.element_id]?;

        if !self.elements.is_empty() {
            write![f, ", Elements:"]?;

            for entry in &self.elements {
                write![f, " {}", entry]?;
            }
        }

        if !self.frames.is_empty() {
            write![f, ", Sub-Frames:"]?;

            for frame in self.frames.values() {
                write![f, " {}", frame.id()]?;
            }
        }

        Ok(())
    }
}

impl Default for TableOfContentsFrame {
    fn default() -> Self {
        Self {
            element_id: String::new(),
            flags: TocFlags::default(),
            elements: Vec::new(),
            frames: FrameMap::new(),
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct TocFlags {
    pub top_level: bool,
    pub ordered: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::tag::Version;

    const EMPTY_CHAP: &[u8] = b"chp1\0\
                                \x00\x00\x00\x00\
                                \x00\x0A\xBC\xDE\
                                \x16\x16\x16\x16\
                                \xFF\xFF\xFF\xFF";

    const FULL_CHAP: &[u8] = b"chp1\0\
                               \x00\x00\x00\x00\
                               \x00\x0A\xBC\xDE\
                               \x16\x16\x16\x16\
                               \xFF\xFF\xFF\xFF\
                               TIT2\x00\x00\x00\x0A\x00\x00\
                               \x00\
                               Chapter 1\
                               TALB\x00\x00\x00\x0D\x00\x00\
                               \x00\
                               P\xF0dcast Name";

    const EMPTY_CTOC: &[u8] = b"toc1\0\
                                \x02\x03\
                                chp1\0chp2\0chp3\0";

    const FULL_CTOC: &[u8] = b"toc1\0\
                               \x01\x03\
                               chp1\0chp2\0chp3\0\
                               TIT2\x00\x00\x00\x07\x00\x00\
                               \x00\
                               P\xE4rt 1\
                               TALB\x00\x00\x00\x0D\x00\x00\
                               \x00\
                               Podcast Name";
    #[test]
    fn parse_chap() {
        let frame = ChapterFrame::parse(
            &TagHeader::with_version(Version::V24),
            &mut BufStream::new(EMPTY_CHAP),
        )
        .unwrap();

        assert_eq!(frame.element_id, "chp1");
        assert_eq!(frame.time.start_time, 0);
        assert_eq!(frame.time.end_time, 0xABCDE);
        assert_eq!(frame.time.start_offset, 0x16161616);
        assert_eq!(frame.time.end_offset, 0xFFFFFFFF);
        assert!(frame.frames.is_empty())
    }

    #[test]
    fn parse_chap_with_frames() {
        let frame = ChapterFrame::parse(
            &TagHeader::with_version(Version::V24),
            &mut BufStream::new(FULL_CHAP),
        )
        .unwrap();

        assert_eq!(frame.element_id, "chp1");
        assert_eq!(frame.time.start_time, 0);
        assert_eq!(frame.time.end_time, 0xABCDE);
        assert_eq!(frame.time.start_offset, 0x16161616);
        assert_eq!(frame.time.end_offset, 0xFFFFFFFF);

        assert_eq!(frame.frames["TIT2"].to_string(), "Chapter 1");
        assert_eq!(frame.frames["TALB"].to_string(), "Pðdcast Name");
    }

    #[test]
    fn parse_ctoc() {
        let frame = TableOfContentsFrame::parse(
            &TagHeader::with_version(Version::V24),
            &mut BufStream::new(EMPTY_CTOC),
        )
        .unwrap();

        assert_eq!(frame.element_id, "toc1");
        assert_eq!(frame.elements, &["chp1", "chp2", "chp3"]);
        assert!(frame.flags.top_level);
        assert!(!frame.flags.ordered);
        assert!(frame.frames.is_empty())
    }

    #[test]
    fn parse_ctoc_with_frames() {
        let frame = TableOfContentsFrame::parse(
            &TagHeader::with_version(Version::V24),
            &mut BufStream::new(FULL_CTOC),
        )
        .unwrap();

        assert_eq!(frame.element_id, "toc1");
        assert_eq!(frame.elements, &["chp1", "chp2", "chp3"]);
        assert!(!frame.flags.top_level);
        assert!(frame.flags.ordered);

        assert_eq!(frame.frames["TIT2"].to_string(), "Pärt 1");
        assert_eq!(frame.frames["TALB"].to_string(), "Podcast Name");
    }
}
