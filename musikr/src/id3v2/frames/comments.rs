//! Comments frames.

use crate::core::io::BufStream;
use crate::id3v2::frames::{encoding, Frame, FrameId, Language};
use crate::id3v2::{ParseResult, TagHeader};
use crate::core::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

#[derive(Default, Debug, Clone)]
pub struct CommentsFrame {
    pub encoding: Encoding,
    pub lang: Language,
    pub desc: String,
    pub text: String,
}

impl CommentsFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let lang = Language::try_new(&stream.read_array()?).unwrap_or_default();
        let desc = string::read_terminated(encoding, stream);
        let text = string::read(encoding, stream);

        Ok(Self {
            encoding,
            lang,
            desc,
            text,
        })
    }
}

impl Frame for CommentsFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"COMM")
    }

    fn key(&self) -> String {
        format!["COMM:{}:{}", self.desc, self.lang]
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

impl Display for CommentsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.text]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMM_DATA: &[u8] = b"COMM\x00\x00\x00\x14\x00\x00\
                                \x03\
                                eng\
                                Description\x00\
                                Text";

    #[test]
    fn parse_comm() {
        make_frame!(CommentsFrame, COMM_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Utf8);
        assert_eq!(frame.lang, b"eng");
        assert_eq!(frame.desc, "Description");
        assert_eq!(frame.text, "Text");
    }

    #[test]
    fn render_comm() {
        let frame = CommentsFrame {
            encoding: Encoding::Utf8,
            lang: Language::new(b"eng"),
            desc: String::from("Description"),
            text: String::from("Text"),
        };

        assert_render!(frame, COMM_DATA);
    }
}
