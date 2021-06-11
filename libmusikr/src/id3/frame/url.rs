use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{FrameHeader, Id3Frame};
use std::fmt::{self, Display, Formatter};

pub struct UrlFrame {
    header: FrameHeader,
    url: String,
}

impl UrlFrame {
    pub(crate) fn new(header: FrameHeader, data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        let url = string::get_string(Encoding::Utf8, data);

        Some(UrlFrame { header, url })
    }

    pub fn from(frame: &dyn Id3Frame) -> Option<&Self> {
        frame.downcast_ref()
    }

    pub fn url(&self) -> &String {
        &self.url
    }
}

impl Id3Frame for UrlFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn key(&self) -> String {
        self.id().clone()
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
    pub(crate) fn new(header: FrameHeader, data: &[u8]) -> Option<Self> {
        let encoding = Encoding::new(*data.get(0)?);

        if data.len() < encoding.nul_size() + 2 {
            return None;
        }

        let (desc, desc_size) = string::get_terminated_string(encoding, &data[1..]);

        let text_pos = 1 + desc_size;
        let url = string::get_string(Encoding::Utf8, &data[text_pos..]);

        Some(UserUrlFrame {
            header,
            encoding,
            desc,
            url,
        })
    }

    pub fn from(frame: &dyn Id3Frame) -> Option<&Self> {
        frame.downcast_ref()
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn url(&self) -> &String {
        &self.url
    }
}

impl Id3Frame for UserUrlFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn key(&self) -> String {
        format!["{}:{}", self.id(), self.desc]
    }
}

impl Display for UserUrlFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.url]
    }
}
