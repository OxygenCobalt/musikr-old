use crate::id3::frame::string;
use crate::id3::frame::string::Encoding;
use crate::id3::frame::Id3Frame;
use crate::id3::frame::Id3FrameHeader;

pub struct TextFrame {
    header: Id3FrameHeader,
    pub encoding: Encoding,
    pub text: String,
}

impl TextFrame {
    pub fn from(header: Id3FrameHeader, data: &[u8]) -> TextFrame {
        let encoding = Encoding::from(data[0]);
        let text = string::get_string(&encoding, &data[1..]);

        return TextFrame {
            header,
            encoding,
            text,
        };
    }
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
