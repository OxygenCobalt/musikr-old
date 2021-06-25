use crate::core::io::BufStream;
use crate::id3v2::frames::lang::Language;
use crate::id3v2::frames::{encoding, Frame, FrameFlags, FrameHeader, Token};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

pub struct CommentsFrame {
    header: FrameHeader,
    encoding: Encoding,
    lang: Language,
    desc: String,
    text: String,
}

impl CommentsFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags(b"COMM", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        CommentsFrame {
            header,
            encoding: Encoding::default(),
            lang: Language::default(),
            desc: String::new(),
            text: String::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::read(stream)?;
        let lang = Language::parse(stream).unwrap_or_default();
        let desc = string::read_terminated(encoding, stream);
        let text = string::read(encoding, stream);

        Ok(CommentsFrame {
            header,
            encoding,
            lang,
            desc,
            text,
        })
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn lang(&self) -> &Language {
        &self.lang
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn text(&self) -> &String {
        &self.text
    }

    pub fn encoding_mut(&mut self) -> &mut Encoding {
        &mut self.encoding
    }

    pub fn lang_mut(&mut self) -> &mut Language {
        &mut self.lang
    }

    pub fn desc_mut(&mut self) -> &mut String {
        &mut self.desc
    }

    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
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

        let encoding = encoding::check(self.encoding, tag_header.major());
        result.push(encoding::render(encoding));
        result.extend(&self.lang);

        result.extend(string::render_terminated(encoding, &self.desc));
        result.extend(string::render(encoding, &self.text));

        result
    }
}

impl Default for CommentsFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
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

    const COMM_DATA: &[u8] = b"\x03\
                                eng\
                                Description\x00\
                                Text";

    #[test]
    fn parse_comm() {
        let frame = CommentsFrame::parse(FrameHeader::new(b"COMM"), &mut BufStream::new(COMM_DATA))
            .unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf8);
        assert_eq!(frame.lang(), "eng");
        assert_eq!(frame.desc(), "Description");
        assert_eq!(frame.text(), "Text");
    }

    #[test]
    fn render_comm() {
        let mut frame = CommentsFrame::new();

        *frame.encoding_mut() = Encoding::Utf8;
        frame.lang_mut().set(b"eng").unwrap();
        frame.desc_mut().push_str("Description");
        frame.text_mut().push_str("Text");

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), COMM_DATA);
    }
}
