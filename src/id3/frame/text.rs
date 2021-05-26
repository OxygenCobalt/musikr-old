use crate::id3::frame::string;
use crate::id3::frame::string::Encoding;
use crate::id3::frame::Id3Frame;
use crate::id3::frame::Id3FrameHeader;

pub struct TextFrame {
    header: Id3FrameHeader,
    pub encoding: Encoding,
    pub text: String,
}

impl Id3Frame for TextFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }

    fn format(&self) -> String {
        return format!["{}: {}", self.header.code, self.text];
    }
}

impl TextFrame {
    pub fn from<'a>(header: Id3FrameHeader, data: &'a [u8]) -> TextFrame {
        let encoding = string::get_encoding(data[0]);
        let text = string::get_string(&encoding, &data[1..]);

        return TextFrame {
            header,
            encoding,
            text,
        };
    }
}
