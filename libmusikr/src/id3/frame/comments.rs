use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};
use std::fmt::{self, Display, Formatter};

pub struct CommentsFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    lang: String,
    desc: String,
    text: String,
}

impl CommentsFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> CommentsFrame {
        let encoding = Encoding::from_raw(data[0]);

        let lang = String::from_utf8_lossy(&data[1..3]).to_string();
        let (desc, desc_size) = string::get_terminated_string(encoding, &data[4..]);

        let text_pos = 4 + desc_size;
        let text = string::get_string(encoding, &data[text_pos..]);

        CommentsFrame {
            header,
            encoding,
            lang,
            desc,
            text,
        }
    }

    fn desc(&self) -> &String {
        &self.desc
    }

    fn text(&self) -> &String {
        &self.text
    }
}

impl Id3Frame for CommentsFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
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
