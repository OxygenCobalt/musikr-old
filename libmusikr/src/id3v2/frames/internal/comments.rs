use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader, ParseError};
use std::fmt::{self, Display, Formatter};

pub struct CommentsFrame {
    header: FrameHeader,
    encoding: Encoding,
    lang: String,
    desc: String,
    text: String,
}

impl CommentsFrame {
    pub fn new(header: FrameHeader) -> Self {
        CommentsFrame {
            header,
            encoding: Encoding::default(),
            lang: String::new(),
            desc: String::new(),
            text: String::new(),
        }
    }

    fn desc(&self) -> &String {
        &self.desc
    }

    fn text(&self) -> &String {
        &self.text
    }
}

impl Frame for CommentsFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn flags(&self) -> &FrameFlags {
        &self.header.flags
    }

    fn key(&self) -> String {
        format!["{}:{}:{}", self.id(), self.desc, self.lang]
    }

    fn parse(&mut self, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < (self.encoding.nul_size() + 5) {
            return Err(ParseError::NotEnoughData);
        }

        self.lang = string::get_string(Encoding::Utf8, &data[1..4]);

        let desc = string::get_terminated_string(self.encoding, &data[4..]);
        self.desc = desc.string;

        let text_pos = 4 + desc.size;
        self.text = string::get_string(self.encoding, &data[text_pos..]);

        Ok(())
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
