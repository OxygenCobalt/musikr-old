use std::fmt::{self, Display, Formatter};

use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};

pub struct TextFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    text: String,
}

impl TextFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> TextFrame {
        let encoding = Encoding::from(data[0]);
        let text = string::get_string(&encoding, &data[1..]);

        return TextFrame {
            header,
            encoding,
            text,
        };
    }

    pub fn text(&self) -> &String {
        return &self.text;
    }
}

impl Id3Frame for TextFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }
}

impl Display for TextFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.text]
    }
}

pub struct UserTextFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    desc: String,
    text: String
}

impl UserTextFrame {
    pub(super) fn from<'a>(header: Id3FrameHeader, data: &[u8]) -> UserTextFrame {
        let encoding = Encoding::from(data[0]);

        let desc = string::get_nul_string(&encoding, &data[1..])
            .unwrap_or_default();

        let text_pos = desc.len() + encoding.get_nul_size();
        let text = string::get_string(&encoding, &data[text_pos..]);

        return UserTextFrame {
            header, encoding, desc, text
        }
    }

    pub fn desc(&self) -> &String {
        return &self.desc;
    }

    pub fn text(&self) -> &String {
        return &self.text;
    }
}

impl Id3Frame for UserTextFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }
}

impl Display for UserTextFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.text]
    }
}
