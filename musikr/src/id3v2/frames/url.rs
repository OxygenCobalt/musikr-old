use crate::core::io::BufStream;
use crate::id3v2::frames::{encoding, Frame, FrameId};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct UrlFrame {
    frame_id: FrameId,
    pub url: String,
}

impl UrlFrame {
    pub fn new(frame_id: FrameId) -> Self {
        // Apple's WFED [Podcast URL] is actually a text frame despite its ID, so it must
        // be disallowed.
        if !frame_id.starts_with(b'W') || matches!(frame_id.inner(), b"WFED" | b"WXXX") {
            panic!("Expected a valid URL frame id, found {}", frame_id)
        }

        Self {
            frame_id,
            url: String::new(),
        }
    }

    pub(crate) fn parse(frame_id: FrameId, stream: &mut BufStream) -> ParseResult<Self> {
        let url = string::read(Encoding::Utf8, stream);

        Ok(Self { frame_id, url })
    }
}

impl Frame for UrlFrame {
    fn id(&self) -> FrameId {
        self.frame_id
    }

    fn key(&self) -> String {
        self.id().to_string()
    }

    fn is_empty(&self) -> bool {
        self.url.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        string::render(Encoding::Latin1, &self.url)
    }
}

impl Display for UrlFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.url]
    }
}

#[derive(Debug, Clone)]
pub struct UserUrlFrame {
    pub encoding: Encoding,
    pub desc: String,
    pub url: String,
}

impl UserUrlFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let desc = string::read_terminated(encoding, stream);
        let url = string::read(Encoding::Latin1, stream);

        Ok(Self {
            encoding,
            desc,
            url,
        })
    }
}

impl Frame for UserUrlFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"WXXX")
    }

    fn key(&self) -> String {
        format!["WXXX:{}", self.desc]
    }

    fn is_empty(&self) -> bool {
        self.url.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.version());
        result.push(encoding::render(self.encoding));

        result.extend(string::render_terminated(encoding, &self.desc));
        result.extend(string::render(Encoding::Latin1, &self.url));

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
        Self {
            encoding: Encoding::default(),
            desc: String::new(),
            url: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::tag::Version;

    const URL_DATA: &[u8] = b"https://fourtet.net";

    const WXXX_DATA: &[u8] = b"\x03\
                               ID3v2.3.0\0\
                               https://id3.org/id3v2.3.0";

    #[test]
    fn parse_url() {
        let frame = UrlFrame::parse(FrameId::new(b"WOAR"), &mut BufStream::new(URL_DATA)).unwrap();

        assert_eq!(frame.url, "https://fourtet.net");
    }

    #[test]
    fn parse_wxxx() {
        let frame = UserUrlFrame::parse(&mut BufStream::new(WXXX_DATA)).unwrap();

        assert_eq!(frame.encoding, Encoding::Utf8);
        assert_eq!(frame.desc, "ID3v2.3.0");
        assert_eq!(frame.url, "https://id3.org/id3v2.3.0");
    }

    #[test]
    fn render_url() {
        let mut frame = UrlFrame::new(FrameId::new(b"WOAR"));
        frame.url.push_str("https://fourtet.net");

        assert!(!frame.is_empty());
        assert_eq!(
            frame.render(&TagHeader::with_version(Version::V24)),
            URL_DATA
        );
    }

    #[test]
    fn render_wxxx() {
        let mut frame = UserUrlFrame::new();

        frame.encoding = Encoding::Utf8;
        frame.desc.push_str("ID3v2.3.0");
        frame.url.push_str("https://id3.org/id3v2.3.0");

        assert!(!frame.is_empty());
        assert_eq!(
            frame.render(&TagHeader::with_version(Version::V24)),
            WXXX_DATA
        );
    }
}
