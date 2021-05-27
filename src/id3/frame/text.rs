use crate::id3::frame::string;
use crate::id3::frame::string::Encoding;
use crate::id3::frame::Id3Frame;
use crate::id3::frame::Id3FrameHeader;

pub struct TextFrame {
    header: Id3FrameHeader,
    pub encoding: Encoding,
    pub text: String,
}

pub struct UserTextFrame {
    header: Id3FrameHeader,
    pub encoding: Encoding,
    pub desc: String,
    pub text: String
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

impl UserTextFrame {
    pub fn from<'a>(header: Id3FrameHeader, data: &[u8]) -> UserTextFrame {
        let encoding = Encoding::from(data[0]);

        let desc = string::get_nul_string(&encoding, &data[1..])
            .unwrap_or_default();

        let text_pos = desc.len() + encoding.get_nul_size();
        let text = string::get_string(&encoding, &data[text_pos..]);

        return UserTextFrame {
            header, encoding, desc, text
        }
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

impl Id3Frame for UserTextFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }

    fn format(&self) -> String {
        return format!["{}: {}: {}", self.header.code, self.desc, self.text];
    }
}
