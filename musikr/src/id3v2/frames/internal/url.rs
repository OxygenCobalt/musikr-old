use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::ParseError;
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

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
        if data.is_empty() {
            // Data cannot be empty
            return Err(ParseError::NotEnoughData);
        }

        let url = string::get_string(Encoding::Latin1, data);

        Ok(UrlFrame { header, url })
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

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
        let encoding = Encoding::parse(data)?;

        if data.len() < encoding.nul_size() + 2 {
            // Must be at least 1 encoding byte, an empty descriptor, and one url byte.
            return Err(ParseError::NotEnoughData);
        }

        let desc = string::get_terminated_string(encoding, &data[1..]);
        let url = string::get_string(Encoding::Latin1, &data[1 + desc.size..]);

        Ok(UserUrlFrame {
            header,
            encoding,
            desc: desc.string,
            url,
        })
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url() {
        let data = b"https://fourtet.net";
        let frame = UrlFrame::parse(FrameHeader::new("WOAR"), &data[..]).unwrap();

        assert_eq!(frame.url(), "https://fourtet.net");
    }

    #[test]
    fn parse_wxxx() {
        let data = b"\x03\
                     ID3v2.3.0\0\
                     https://id3.org/id3v2.3.0";

        let frame = UserUrlFrame::parse(FrameHeader::new("WXXX"), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf8);
        assert_eq!(frame.desc(), "ID3v2.3.0");
        assert_eq!(frame.url(), "https://id3.org/id3v2.3.0");
    }
}
