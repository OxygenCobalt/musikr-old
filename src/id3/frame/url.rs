use std::fmt::{self, Display, Formatter};

use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};

pub struct UrlFrame {
    header: Id3FrameHeader,
    url: String,
}

impl UrlFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> UrlFrame {
        let url = string::get_string(&Encoding::Utf8, &data[0..]);

        return UrlFrame { header, url };
    }

    pub fn url(&self) -> &String {
        return &self.url;
    }
}

impl Id3Frame for UrlFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }
}

impl Display for UrlFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.url]
    }
}

pub struct UserUrlFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    desc: String,
    url: String,
}

impl UserUrlFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> UserUrlFrame {
        let encoding = Encoding::from(data[0]);
        let desc = string::get_nul_string(&encoding, &data[1..]).unwrap_or_default();
        let text_pos = desc.len() + encoding.get_nul_size();
        let url = string::get_string(&Encoding::Utf8, &data[text_pos..]);

        return UserUrlFrame {
            header,
            encoding,
            desc,
            url,
        };
    }

    pub fn desc(&self) -> &String {
        return &self.desc;
    }

    pub fn url(&self) -> &String {
        return &self.url;
    }
}

impl Id3Frame for UserUrlFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }
}

impl Display for UserUrlFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.url]
    }
}
