use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::{ParseError, TagHeader};
use std::fmt::{self, Display, Formatter};

pub struct UrlFrame {
    header: FrameHeader,
    url: String,
}

impl UrlFrame {
    pub fn new(frame_id: &str) -> Self {
        Self::with_flags(frame_id, FrameFlags::default())
    }

    pub fn with_flags(frame_id: &str, flags: FrameFlags) -> Self {
        if !frame_id.starts_with('W') {
            panic!("UrlFrame IDs must start with a W.")
        }

        if frame_id == "WXXX" {
            panic!("UrlFrame cannot encode WXXX frames, use UserUrlFrame instead.")
        }

        // Apple's WFED [Podcast URL] is a weird hybrid between a text frame and a URL frame.
        // To prevent a trivial mistake that could break this tag, we disallow this frame
        // from being encoded in a UrlFrame.
        if frame_id == "WFED" {
            panic!("UrlFrame cannot encode iTunes WFED frames, use TextFrame instead.")
        }

        Self::with_header(FrameHeader::with_flags(frame_id, flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        UrlFrame {
            header,
            url: String::new(),
        }
    }

    pub fn url(&self) -> &String {
        &self.url
    }
}

impl Frame for UrlFrame {
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
        if data.is_empty() {
            return Err(ParseError::NotEnoughData);
        }

        self.url = string::get_string(Encoding::Latin1, data);

        Ok(())
    }
}

impl Display for UrlFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.url]
    }
}

pub struct UserUrlFrame {
    header: FrameHeader,
    encoding: Encoding,
    desc: String,
    url: String,
}

impl UserUrlFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("WXXX", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        UserUrlFrame {
            header,
            encoding: Encoding::default(),
            desc: String::new(),
            url: String::new(),
        }
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn url(&self) -> &String {
        &self.url
    }
}

impl Frame for UserUrlFrame {
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
        format!["{}:{}", self.id(), self.desc]
    }

    fn parse(&mut self, _header: &TagHeader, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < self.encoding.nul_size() + 2 {
            return Err(ParseError::NotEnoughData); // Not enough data
        }

        let desc = string::get_terminated_string(self.encoding, &data[1..]);
        self.desc = desc.string;

        let text_pos = 1 + desc.size;
        self.url = string::get_string(Encoding::Latin1, &data[text_pos..]);

        Ok(())
    }
}

impl Display for UserUrlFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.url]
    }
}

impl Default for UserUrlFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}
