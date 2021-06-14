use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{self, FrameMap, Frame, FrameFlags, FrameHeader};
use crate::id3v2::{TagHeader, ParseError};
use crate::raw;
use std::fmt::{self, Display, Formatter};

pub struct ChapterFrame {
    header: FrameHeader,
    element_id: String,
    start_time: u32,
    end_time: u32,
    start_offset: u32,
    end_offset: u32,
    frames: FrameMap
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
            start_time: 0,
            end_time: 0,
            start_offset: u32::MAX,
            end_offset: u32::MAX,
            frames: FrameMap::new()
        }
    }

    pub fn element_id(&self) -> &String {
        &self.element_id
    }

    pub fn start_time(&self) -> u32 {
        self.start_time
    }

    pub fn end_time(&self) -> u32 {
        self.end_time
    }

    pub fn start_offset(&self) -> u32 {
        self.start_offset
    }

    pub fn end_offset(&self) -> u32 {
        self.end_offset
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

    fn parse(&mut self, header: &TagHeader, data: &[u8]) -> Result<(), ParseError> {
        if data.len() < 17 {
            return Err(ParseError::NotEnoughData)
        }

        let elem_id = string::get_terminated_string(Encoding::Utf8, data);
        self.element_id = elem_id.string;
        
        let time_pos = elem_id.size;
        self.start_time = raw::to_u32(&data[time_pos..time_pos + 4]);
        self.end_time = raw::to_u32(&data[time_pos + 4..time_pos + 8]);
        self.start_offset = raw::to_u32(&data[time_pos + 8..time_pos + 12]);
        self.end_offset = raw::to_u32(&data[time_pos + 12..time_pos + 16]);

        // Embedded frames are optional.

        let mut frame_pos = elem_id.size + 16;

        while frame_pos <= data.len() {
            // Recursively call frames::new until we run out of space. All rules from the tag header
            // must be applied to chapter sub-frames.
            let frame = match frames::new(header, &data[frame_pos..]) {
                Ok(frame) => frame,
                Err(_) => break,
            };

            // Add our new frame.
            frame_pos += frame.size() + 10;
            self.frames.add(frame);
        }

        Ok(())
    }
}

impl Display for ChapterFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{} [Start: {}, End: {}]", self.element_id, self.start_time, self.end_time]?;
        
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