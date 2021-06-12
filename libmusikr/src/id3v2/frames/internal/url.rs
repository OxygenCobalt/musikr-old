use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader, ParseError};
use std::fmt::{self, Display, Formatter};

pub struct UrlFrame {
    header: FrameHeader,
    url: String,
}

impl UrlFrame {
    pub fn new(header: FrameHeader) -> Self {
        UrlFrame {
            header,
            url: String::new(),
        }
    }

    pub fn url(&self) -> &String {
        &self.url
    }
}

impl Frame for UrlFrame {
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
        self.id().clone()
    }

    fn parse(&mut self, data: &[u8]) -> Result<(), ParseError> {
        if data.is_empty() {
            return Err(ParseError::NotEnoughData);
        }

        self.url = string::get_string(Encoding::Utf8, data);

        Ok(())
    }
}

impl Display for UrlFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.url]
    }
}

pub struct UserUrlFrame {
    header: FrameHeader,
    encoding: Encoding,
    desc: String,
    url: String,
}

impl UserUrlFrame {
    pub fn new(header: FrameHeader) -> Self {
        UserUrlFrame {
            header,
            encoding: Encoding::default(),
            desc: String::new(),
            url: String::new(),
        }
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn url(&self) -> &String {
        &self.url
    }
}

impl Frame for UserUrlFrame {
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
        format!["{}:{}", self.id(), self.desc]
    }

    fn parse(&mut self, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < self.encoding.nul_size() + 2 {
            return Err(ParseError::NotEnoughData); // Not enough data
        }

        let desc = string::get_terminated_string(self.encoding, &data[1..]);
        self.desc = desc.string;

        let text_pos = 1 + desc.size;
        self.url = string::get_string(Encoding::Utf8, &data[text_pos..]);

        Ok(())
    }
}

impl Display for UserUrlFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.url]
    }
}
