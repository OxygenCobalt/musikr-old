use crate::core::io::BufStream;
use crate::id3v2::frames::{self, Frame, FrameId, FrameResult};
use crate::id3v2::{FrameMap, ParseResult, TagHeader};
use crate::string::{self, Encoding};
use log::{info, warn};
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;

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

        let frames = parse_embedded_frames(tag_header, stream);

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

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend(string::render_terminated(
            Encoding::Latin1,
            &self.element_id,
        ));

        result.extend(self.time.start_time.to_be_bytes());
        result.extend(self.time.end_time.to_be_bytes());
        result.extend(self.time.start_offset.to_be_bytes());
        result.extend(self.time.end_offset.to_be_bytes());
        result.extend(render_embedded_frames(tag_header, &self.frames));

        result
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

        // Embedded frames come after the entry list
        let frames = parse_embedded_frames(tag_header, stream);

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
        self.elements.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend(string::render_terminated(
            Encoding::Latin1,
            &self.element_id,
        ));

        let mut flags = 0;
        flags |= u8::from(self.flags.top_level) * 0x2;
        flags |= u8::from(self.flags.ordered);

        result.push(flags);

        // Truncate the element count to 256. Not worth throwing an error.
        let element_count = usize::min(self.elements.len(), u8::MAX as usize);

        if element_count != self.elements.len() {
            warn!(
                target: "id3v2:ctoc",
                "cannot encode {} elements in {}, truncating to 255.",
                self.elements.len(), self.element_id
            )
        }

        result.push(element_count as u8);

        for i in 0..element_count {
            result.extend(string::render_terminated(
                Encoding::Latin1,
                &self.elements[i],
            ))
        }

        result.extend(render_embedded_frames(tag_header, &self.frames));

        result
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

fn parse_embedded_frames(tag_header: &TagHeader, stream: &mut BufStream) -> FrameMap {
    let mut frames = FrameMap::new();

    while let Ok(result) = frames::parse(&tag_header, stream) {
        match result {
            FrameResult::Frame(frame) => frames.add(frame),
            FrameResult::Unknown(unknown) => {
                // Drop unknown frames if theyre encountered. This is mostly for simplicity, as this
                // allows all members in a ChapterFrame/TableOfContentsFrame to be public and also
                // avoid having to deal with unknown frames during an upgrade.
                info!(target: "id3v2:chap/ctoc", "dropping unknown frame {}", unknown.id());
            }
            FrameResult::Empty => {
                // Empty frames have already moved the stream to the next
                // frame, so we can skip it.
            }
        }
    }

    frames
}

fn render_embedded_frames(tag_header: &TagHeader, frames: &FrameMap) -> Vec<u8> {
    let mut result = Vec::new();

    for frame in frames.values() {
        if !frame.is_empty() {
            // Its better to just drop frames that are error here than propagate the error.
            // CHAP and CTOC already break musikr's abstractions enough.
            if let Ok(data) = frames::render(tag_header, frame.deref()) {
                result.extend(data)
            } else {
                warn!(target: "id3v2:chap/ctoc", "could not render frame {}", frame.key())
            }
        } else {
            info!(target: "id3v2:chap/ctoc", "dropping empty frame {}", frame.key())
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::frames::TextFrame;

    const EMPTY_CHAP: &[u8] = b"CHAP\x00\x00\x00\x15\x00\x00\
                                chp1\0\
                                \x00\x00\x00\x00\
                                \x00\x0A\xBC\xDE\
                                \x16\x16\x16\x16\
                                \xFF\xFF\xFF\xFF";

    const FULL_CHAP: &[u8] = b"CHAP\x00\x00\x00\x40\x00\x00\
                               chp1\0\
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

    const EMPTY_CTOC: &[u8] = b"CTOC\x00\x00\x00\x16\x00\x00\
                                toc1\0\
                                \x02\x03\
                                chp1\0chp2\0chp3\0";

    const FULL_CTOC: &[u8] = b"CTOC\x00\x00\x00\x3E\x00\x00\
                               toc1\0\
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
        crate::make_frame!(ChapterFrame, EMPTY_CHAP, frame);

        assert_eq!(frame.element_id, "chp1");
        assert_eq!(
            frame.time,
            ChapterTime {
                start_time: 0,
                end_time: 0xABCDE,
                start_offset: 0x16161616,
                end_offset: 0xFFFFFFFF
            }
        );
        assert!(frame.frames.is_empty())
    }

    #[test]
    fn parse_chap_with_frames() {
        crate::make_frame!(ChapterFrame, FULL_CHAP, frame);

        assert_eq!(frame.element_id, "chp1");
        assert_eq!(
            frame.time,
            ChapterTime {
                start_time: 0,
                end_time: 0xABCDE,
                start_offset: 0x16161616,
                end_offset: 0xFFFFFFFF
            }
        );

        assert_eq!(frame.frames["TIT2"].to_string(), "Chapter 1");
        assert_eq!(frame.frames["TALB"].to_string(), "Pðdcast Name");
    }

    #[test]
    fn parse_ctoc() {
        crate::make_frame!(TableOfContentsFrame, EMPTY_CTOC, frame);

        assert_eq!(frame.element_id, "toc1");
        assert_eq!(frame.elements, &["chp1", "chp2", "chp3"]);
        assert!(frame.flags.top_level);
        assert!(!frame.flags.ordered);
        assert!(frame.frames.is_empty())
    }

    #[test]
    fn parse_ctoc_with_frames() {
        crate::make_frame!(TableOfContentsFrame, FULL_CTOC, frame);

        assert_eq!(frame.element_id, "toc1");
        assert_eq!(frame.elements, &["chp1", "chp2", "chp3"]);
        assert!(!frame.flags.top_level);
        assert!(frame.flags.ordered);

        assert_eq!(frame.frames["TIT2"].to_string(), "Pärt 1");
        assert_eq!(frame.frames["TALB"].to_string(), "Podcast Name");
    }

    #[test]
    fn render_chap() {
        let frame = ChapterFrame {
            element_id: String::from("chp1"),
            time: ChapterTime {
                start_time: 0,
                end_time: 0xABCDE,
                start_offset: 0x16161616,
                end_offset: 0xFFFFFFFF,
            },
            frames: FrameMap::new(),
        };

        crate::assert_render!(frame, EMPTY_CHAP);
    }

    #[test]
    fn render_chap_with_frames() {
        let mut frame = ChapterFrame {
            element_id: String::from("chp1"),
            time: ChapterTime {
                start_time: 0,
                end_time: 0xABCDE,
                start_offset: 0x16161616,
                end_offset: 0xFFFFFFFF,
            },
            frames: FrameMap::new(),
        };

        let mut talb = TextFrame::new(FrameId::new(b"TALB"));
        talb.encoding = Encoding::Latin1;
        talb.text = vec![String::from("Pðdcast Name")];

        let mut tit2 = TextFrame::new(FrameId::new(b"TIT2"));
        tit2.encoding = Encoding::Latin1;
        tit2.text = vec![String::from("Chapter 1")];

        frame.frames.insert(Box::new(tit2));
        frame.frames.insert(Box::new(talb));

        crate::assert_render!(frame, FULL_CHAP);
    }

    #[test]
    fn render_ctoc() {
        let frame = TableOfContentsFrame {
            element_id: String::from("toc1"),
            elements: vec![
                String::from("chp1"),
                String::from("chp2"),
                String::from("chp3"),
            ],
            flags: TocFlags {
                top_level: true,
                ordered: false,
            },
            frames: FrameMap::new(),
        };

        crate::assert_render!(frame, EMPTY_CTOC);
    }

    #[test]
    fn render_ctoc_with_frames() {
        let mut frame = TableOfContentsFrame {
            element_id: String::from("toc1"),
            elements: vec![
                String::from("chp1"),
                String::from("chp2"),
                String::from("chp3"),
            ],
            flags: TocFlags {
                top_level: false,
                ordered: true,
            },
            frames: FrameMap::new(),
        };

        let mut tit2 = TextFrame::new(FrameId::new(b"TIT2"));
        tit2.encoding = Encoding::Latin1;
        tit2.text = vec![String::from("Pärt 1")];

        let mut talb = TextFrame::new(FrameId::new(b"TALB"));
        talb.encoding = Encoding::Latin1;
        talb.text = vec![String::from("Podcast Name")];

        frame.frames.insert(Box::new(tit2));
        frame.frames.insert(Box::new(talb));

        crate::assert_render!(frame, FULL_CTOC);
    }
}
