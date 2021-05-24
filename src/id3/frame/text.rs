use super::FrameHeader;
use super::ID3Frame;
use super::string;
use super::string::ID3Encoding;

pub struct TextFrame {
    header: FrameHeader,
    pub encoding: ID3Encoding,
    pub text: String
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
            header, encoding, text
        };
    }
}
