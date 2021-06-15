use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::{ParseError, TagHeader};
use std::fmt::{self, Display, Formatter};

pub struct PodcastFrame {
    header: FrameHeader,
}

impl PodcastFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("PCST", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        PodcastFrame { header }
    }
}

impl Frame for PodcastFrame {
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
        self.id().clone()
    }

    fn parse(&mut self, _header: &TagHeader, data: &[u8]) -> Result<(), ParseError> {
        // The iTunes podcast frame is for some reason just four zeroes, meaning that this
        // frames existence is pretty much the only form of mutability it has. Therefore
        // we just validate the given data and move on.
        if data != b"\0\0\0\0" {
            return Err(ParseError::InvalidData);
        }

        Ok(())
    }
}

impl Display for PodcastFrame {
    fn fmt(&self, _f: &mut Formatter) -> fmt::Result {
        // Nothing to format.
        Ok(())
    }
}

impl Default for PodcastFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}