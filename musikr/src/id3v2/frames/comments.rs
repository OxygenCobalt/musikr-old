use crate::err::{ParseError, ParseResult};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::TagHeader;
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

pub struct CommentsFrame {
    header: FrameHeader,
    encoding: Encoding,
    lang: String,
    desc: String,
    text: String,
}

impl CommentsFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("COMM", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        CommentsFrame {
            header,
            encoding: Encoding::default(),
            lang: String::new(),
            desc: String::new(),
            text: String::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> ParseResult<Self> {
        let encoding = Encoding::parse(data)?;

        if data.len() < (encoding.nul_size() + 4) {
            // Must be at least an empty descriptor and 3 bytes for the language.
            return Err(ParseError::NotEnoughData);
        }

        let lang = string::get_string(Encoding::Latin1, &data[1..4]);
        let desc = string::get_terminated(encoding, &data[4..]);
        let text = string::get_string(encoding, &data[4 + desc.size..]);

        Ok(CommentsFrame {
            header,
            encoding,
            lang,
            desc: desc.string,
            text,
        })
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn lang(&self) -> &String {
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

    pub fn lang_mut(&mut self) -> &mut String {
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
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        &self.header.flags()
    }

    fn key(&self) -> String {
        format!["{}:{}:{}", self.id(), self.desc, self.lang]
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = self.encoding.map_id3v2(tag_header.major());
        result.push(encoding.render());

        if self.lang.len() == 3 {
            result.extend(string::render_string(Encoding::Latin1, &self.lang))
        } else {
            result.extend(b"xxx")
        }

        result.extend(string::render_terminated(encoding, &self.desc));
        result.extend(string::render_string(encoding, &self.text));

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

    #[test]
    fn parse_comm() {
        let data = b"\x03\
                     eng\
                     Description\x00\
                     Text";

        let frame = CommentsFrame::parse(FrameHeader::new("COMM"), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf8);
        assert_eq!(frame.lang(), "eng");
        assert_eq!(frame.desc(), "Description");
        assert_eq!(frame.text(), "Text");
    }

    #[test]
    fn render_comm() {
        let out = b"\x03\
                    eng\
                    Description\x00\
                    Text";

        let mut frame = CommentsFrame::new();

        *frame.encoding_mut() = Encoding::Utf8;
        frame.lang_mut().push_str("eng");
        frame.desc_mut().push_str("Description");
        frame.text_mut().push_str("Text");

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), out);
    }
}