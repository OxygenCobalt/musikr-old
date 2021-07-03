use crate::core::io::BufStream;
use crate::id3v2::frames::lang::Language;
use crate::id3v2::frames::{encoding, Frame, FrameHeader, FrameId, Token};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct CommentsFrame {
    header: FrameHeader,
    pub encoding: Encoding,
    pub lang: Language,
    pub desc: String,
    pub text: String,
}

impl CommentsFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let lang = Language::parse(&stream.read_array()?).unwrap_or_default();
        let desc = string::read_terminated(encoding, stream);
        let text = string::read(encoding, stream);

        Ok(Self {
            header,
            encoding,
            lang,
            desc,
            text,
        })
    }
}

impl Frame for CommentsFrame {
    fn key(&self) -> String {
        format!["COMM:{}:{}", self.desc, self.lang]
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.version());
        result.push(encoding::render(encoding));
        result.extend(&self.lang);

        result.extend(string::render_terminated(encoding, &self.desc));
        result.extend(string::render(encoding, &self.text));

        result
    }
}

impl Default for CommentsFrame {
    fn default() -> Self {
        Self {
            header: FrameHeader::new(FrameId::new(b"COMM")),
            encoding: Encoding::default(),
            lang: Language::default(),
            desc: String::new(),
            text: String::new(),
        }
    }
}

impl Display for CommentsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.text]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::tag::Version;

    const COMM_DATA: &[u8] = b"\x03\
                                eng\
                                Description\x00\
                                Text";

    #[test]
    fn parse_comm() {
        let frame = CommentsFrame::parse(
            FrameHeader::new(FrameId::new(b"COMM")),
            &mut BufStream::new(COMM_DATA),
        )
        .unwrap();

        assert_eq!(frame.encoding, Encoding::Utf8);
        assert_eq!(frame.lang.code(), b"eng");
        assert_eq!(frame.desc, "Description");
        assert_eq!(frame.text, "Text");
    }

    #[test]
    fn render_comm() {
        let mut frame = CommentsFrame::new();
        frame.encoding = Encoding::Utf8;
        frame.lang = Language::new(b"eng");
        frame.desc.push_str("Description");
        frame.text.push_str("Text");

        assert!(!frame.is_empty());
        assert_eq!(
            frame.render(&TagHeader::with_version(Version::V24)),
            COMM_DATA
        );
    }
}
