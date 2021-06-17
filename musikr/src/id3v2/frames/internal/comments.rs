use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::ParseError;
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

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
        let encoding = Encoding::parse(data)?;

        if data.len() < (encoding.nul_size() + 4) {
            // Must be at least an empty descriptor and 3 bytes for the language.
            return Err(ParseError::NotEnoughData);
        }

        let lang = string::get_string(Encoding::Latin1, &data[1..4]);
        let desc = string::get_terminated_string(encoding, &data[4..]);
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
}

impl Default for CommentsFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}

impl Display for CommentsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Certain taggers [such as kid3] will write to the description field instead of the text
        // field by default, so if that's the case we will print the description instead of the text.
        if self.text.is_empty() {
            write![f, "{}", self.desc]
        } else {
            write![f, "{}", self.text]
        }
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
}
