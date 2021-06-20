use crate::err::{ParseError, ParseResult};
use crate::id3v2::frames::{self, Frame, FrameFlags, FrameHeader};
use crate::id3v2::{FrameMap, TagHeader};
use crate::raw;
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

pub struct ChapterFrame {
    header: FrameHeader,
    element_id: String,
    time: ChapterTime,
    frames: FrameMap,
}

impl ChapterFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("CHAP", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        ChapterFrame {
            header,
            element_id: String::new(),
            time: ChapterTime::default(),
            frames: FrameMap::new(),
        }
    }

    pub(crate) fn parse(
        header: FrameHeader,
        tag_header: &TagHeader,
        data: &[u8],
    ) -> ParseResult<Self> {
        if data.len() < 18 {
            // Must be at least a one-byte element ID followed by 16 bytes of time
            // information.
            return Err(ParseError::NotEnoughData);
        }

        let elem_id = string::get_terminated(Encoding::Latin1, data);

        let time_pos = elem_id.size;
        let time = ChapterTime {
            start_time: raw::to_u32(&data[time_pos..time_pos + 4]),
            end_time: raw::to_u32(&data[time_pos + 4..time_pos + 8]),
            start_offset: raw::to_u32(&data[time_pos + 8..time_pos + 12]),
            end_offset: raw::to_u32(&data[time_pos + 12..time_pos + 16]),
        };

        // Embedded frames are optional.

        let mut frame_pos = elem_id.size + 16;
        let mut frames = FrameMap::new();

        while frame_pos < data.len() {
            // Recursively call frames::new until we run out of space. All rules from the tag header
            // must be applied to chapter sub-frames.
            let frame = match frames::new(tag_header, &data[frame_pos..]) {
                Ok(frame) => frame,
                Err(_) => break,
            };

            // Add our new frame.
            frame_pos += frame.size() + 10;
            frames.add(frame);
        }

        Ok(ChapterFrame {
            header,
            element_id: elem_id.string,
            time,
            frames,
        })
    }

    pub fn time(&self) -> &ChapterTime {
        &self.time
    }

    pub fn element_id(&self) -> &String {
        &self.element_id
    }

    pub fn frames(&self) -> &FrameMap {
        &self.frames
    }

    pub fn frames_mut(&mut self) -> &mut FrameMap {
        &mut self.frames
    }
}

impl Frame for ChapterFrame {
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        format!["{}:{}", self.id(), self.element_id]
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

            for frame in self.frames.frames() {
                write![f, " {}", frame.id()]?;
            }
        }

        Ok(())
    }
}

impl Default for ChapterFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}

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

pub struct TableOfContentsFrame {
    header: FrameHeader,
    element_id: String,
    flags: TocFlags,
    elements: Vec<String>,
    frames: FrameMap,
}

impl TableOfContentsFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("CTOC", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        TableOfContentsFrame {
            header,
            element_id: String::new(),
            flags: TocFlags::default(),
            elements: Vec::new(),
            frames: FrameMap::new(),
        }
    }

    pub(crate) fn parse(
        header: FrameHeader,
        tag_header: &TagHeader,
        data: &[u8],
    ) -> ParseResult<Self> {
        if data.len() < 4 {
            // Must be at least a one-byte element ID and then two bytes for flags and element count
            return Err(ParseError::NotEnoughData);
        }

        let elem_id = string::get_terminated(Encoding::Latin1, data);
        let flags = data[elem_id.size];

        let flags = TocFlags {
            top_level: raw::bit_at(1, flags),
            ordered: raw::bit_at(0, flags),
        };

        let mut elements: Vec<String> = Vec::new();
        let entry_count = data[elem_id.size + 1];
        let mut pos = elem_id.size + 2;
        let mut i = 0;

        // The entry count may be inaccurate, so we also ensure that we don't overread the data.
        while i < entry_count && pos < data.len() {
            let element = string::get_terminated(Encoding::Latin1, &data[pos..]);

            elements.push(element.string);
            pos += element.size;
            i += 1;
        }

        let mut frames = FrameMap::new();

        // Second loop, this time to get any embedded frames.
        while pos < data.len() {
            let frame = match frames::new(tag_header, &data[pos..]) {
                Ok(frame) => frame,
                Err(_) => break,
            };

            // Add our new frame.
            pos += frame.size() + 10;
            frames.add(frame);
        }

        Ok(TableOfContentsFrame {
            header,
            element_id: elem_id.string,
            flags,
            elements,
            frames,
        })
    }

    pub fn element_id(&self) -> &String {
        &self.element_id
    }

    pub fn flags(&self) -> &TocFlags {
        &self.flags
    }

    pub fn elements(&self) -> &Vec<String> {
        &self.elements
    }

    pub fn frames(&self) -> &FrameMap {
        &self.frames
    }

    pub fn frames_mut(&mut self) -> &mut FrameMap {
        &mut self.frames
    }
}

impl Frame for TableOfContentsFrame {
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        format!["{}:{}", self.id(), self.element_id]
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

            for frame in self.frames.frames() {
                write![f, " {}", frame.id()]?;
            }
        }

        Ok(())
    }
}

impl Default for TableOfContentsFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}

pub struct TocFlags {
    pub top_level: bool,
    pub ordered: bool,
}

impl Default for TocFlags {
    fn default() -> Self {
        TocFlags {
            top_level: false,
            ordered: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            FrameHeader::new("CHAP"),
            &TagHeader::with_version(4),
            EMPTY_CHAP,
        )
        .unwrap();

        assert_eq!(frame.element_id(), "chp1");
        assert_eq!(frame.time().start_time, 0);
        assert_eq!(frame.time().end_time, 0xABCDE);
        assert_eq!(frame.time().start_offset, 0x16161616);
        assert_eq!(frame.time().end_offset, 0xFFFFFFFF);
        assert!(frame.frames().is_empty())
    }

    #[test]
    fn parse_chap_with_frames() {
        let frame = ChapterFrame::parse(
            FrameHeader::new("CHAP"),
            &TagHeader::with_version(4),
            FULL_CHAP,
        )
        .unwrap();

        assert_eq!(frame.element_id(), "chp1");
        assert_eq!(frame.time().start_time, 0);
        assert_eq!(frame.time().end_time, 0xABCDE);
        assert_eq!(frame.time().start_offset, 0x16161616);
        assert_eq!(frame.time().end_offset, 0xFFFFFFFF);

        assert_eq!(frame.frames()["TIT2"].to_string(), "Chapter 1");
        assert_eq!(frame.frames()["TALB"].to_string(), "Pðdcast Name");
    }

    #[test]
    fn parse_ctoc() {
        let frame = TableOfContentsFrame::parse(
            FrameHeader::new("CTOC"),
            &TagHeader::with_version(4),
            EMPTY_CTOC,
        )
        .unwrap();

        assert_eq!(frame.element_id(), "toc1");
        assert_eq!(frame.elements(), &["chp1", "chp2", "chp3"]);
        assert!(frame.flags().top_level);
        assert!(!frame.flags().ordered);
        assert!(frame.frames().is_empty())
    }

    #[test]
    fn parse_ctoc_with_frames() {
        let frame = TableOfContentsFrame::parse(
            FrameHeader::new("CTOC"),
            &TagHeader::with_version(4),
            FULL_CTOC,
        )
        .unwrap();

        assert_eq!(frame.element_id(), "toc1");
        assert_eq!(frame.elements(), &["chp1", "chp2", "chp3"]);
        assert!(!frame.flags().top_level);
        assert!(frame.flags().ordered);

        assert_eq!(frame.frames()["TIT2"].to_string(), "Pärt 1");
        assert_eq!(frame.frames()["TALB"].to_string(), "Podcast Name");
    }
}
