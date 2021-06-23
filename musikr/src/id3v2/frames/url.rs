use crate::id3v2::frames::{encoding, Frame, FrameFlags, FrameHeader};
use crate::id3v2::{ParseError, ParseResult, TagHeader, Token};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

pub struct UrlFrame {
    header: FrameHeader,
    url: String,
}

impl UrlFrame {
    pub fn new(frame_id: &[u8; 4]) -> Self {
        Self::with_flags(frame_id, FrameFlags::default())
    }

    pub fn with_flags(frame_id: &[u8; 4], flags: FrameFlags) -> Self {
        if !frame_id.starts_with(&[b'W']) {
            panic!("UrlFrame IDs must start with a W.")
        }

        if frame_id == b"WXXX" {
            panic!("UrlFrame cannot encode WXXX frames, use UserUrlFrame instead.")
        }

        // Apple's WFED [Podcast URL] is a weird hybrid between a text frame and a URL frame.
        // To prevent a trivial mistake that could break this tag, we disallow this frame
        // from being encoded in a UrlFrame.
        if frame_id == b"WFED" {
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

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> ParseResult<Self> {
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

    pub fn url_mut(&mut self) -> &mut String {
        &mut self.url
    }
}

impl Frame for UrlFrame {
    fn key(&self) -> String {
        self.id().to_string()
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.url.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        string::render_string(Encoding::Latin1, &self.url)
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
        Self::with_header(FrameHeader::with_flags(b"WXXX", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        UserUrlFrame {
            header,
            encoding: Encoding::default(),
            desc: String::new(),
            url: String::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> ParseResult<Self> {
        let encoding = encoding::get(data)?;

        if data.len() < encoding.nul_size() + 2 {
            // Must be at least 1 encoding byte, an empty descriptor, and one url byte.
            return Err(ParseError::NotEnoughData);
        }

        let desc = string::get_terminated(encoding, &data[1..]);
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

    pub fn encoding_mut(&mut self) -> &mut Encoding {
        &mut self.encoding
    }

    pub fn desc_mut(&mut self) -> &mut String {
        &mut self.desc
    }

    pub fn url_mut(&mut self) -> &mut String {
        &mut self.url
    }
}

impl Frame for UserUrlFrame {
    fn key(&self) -> String {
        format!["WXXX:{}", self.desc]
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.url.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.major());
        result.push(encoding::render(self.encoding));

        result.extend(string::render_terminated(encoding, &self.desc));
        result.extend(string::render_string(Encoding::Latin1, &self.url));

        result
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

    const URL_DATA: &[u8] = b"https://fourtet.net";

    const WXXX_DATA: &[u8] = b"\x03\
                               ID3v2.3.0\0\
                               https://id3.org/id3v2.3.0";

    #[test]
    fn parse_url() {
        let frame = UrlFrame::parse(FrameHeader::new(b"WOAR"), URL_DATA).unwrap();

        assert_eq!(frame.url(), "https://fourtet.net");
    }

    #[test]
    fn parse_wxxx() {
        let frame = UserUrlFrame::parse(FrameHeader::new(b"WXXX"), WXXX_DATA).unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf8);
        assert_eq!(frame.desc(), "ID3v2.3.0");
        assert_eq!(frame.url(), "https://id3.org/id3v2.3.0");
    }

    #[test]
    fn render_url() {
        let mut frame = UrlFrame::new(b"WOAR");
        frame.url_mut().push_str("https://fourtet.net");

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), URL_DATA);
    }

    #[test]
    fn render_wxxx() {
        let mut frame = UserUrlFrame::new();

        *frame.encoding_mut() = Encoding::Utf8;
        frame.desc_mut().push_str("ID3v2.3.0");
        frame.url_mut().push_str("https://id3.org/id3v2.3.0");

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), WXXX_DATA);
    }
}
