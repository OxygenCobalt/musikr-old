use std::fmt::{self, Display, Formatter};

use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};

pub struct CommentsFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    lang: String,
    desc: String,
    text: String,
}

impl CommentsFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> CommentsFrame {
        let encoding = Encoding::from(data[0]);
        let lang = String::from_utf8_lossy(&data[1..4]).to_string();
        let desc = string::get_nul_string(&encoding, &data[5..]).unwrap_or_default();
        let text = string::get_string(&encoding, &data[5 + desc.len()..]);

        return CommentsFrame {
            header,
            encoding,
            lang,
            desc,
            text
        }
    }
}

impl Id3Frame for CommentsFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }
}

impl Display for CommentsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Certain taggers [such as kid3] will write to the description field instead of the text
        // field, so if that's the case we will write the description instead of the text.
        if self.text == "" {
            write![f, "{}", self.desc]
        } else {
            write![f, "{}", self.text]
        }
    }
}