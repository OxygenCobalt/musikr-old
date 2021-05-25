use crate::id3::frame::string;
use crate::id3::frame::string::ID3Encoding;
use crate::id3::frame::FrameHeader;
use crate::id3::frame::ID3Frame;

pub struct TextFrame {
    header: FrameHeader,
    pub encoding: ID3Encoding,
    pub text: String,
}

impl ID3Frame for TextFrame {
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
    pub fn from(header: FrameHeader, data: &[u8]) -> TextFrame {
        let encoding = string::get_encoding(data[0]);
        let text = string::get_string(&encoding, &data[1..]);

        return TextFrame {
            header,
            encoding,
            text,
        };
    }
}
