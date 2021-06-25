use crate::core::io::BufStream;
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader, Token};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

pub struct UnknownFrame {
    header: FrameHeader,
    data: Vec<u8>,
}

impl UnknownFrame {
    pub(crate) fn from_stream(header: FrameHeader, stream: &mut BufStream) -> Self {
        UnknownFrame {
            header,
            data: stream.take_rest().to_vec(),
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}

impl Frame for UnknownFrame {
    fn key(&self) -> String {
        self.id().to_string()
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        self.data.clone()
    }
}

impl Display for UnknownFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let data = if self.data.len() > 64 {
            // Truncate the hex data to 64 bytes
            &self.data[0..64]
        } else {
            &self.data
        };

        for byte in data {
            write![f, "{:02x}", byte]?;
        }

        Ok(())
    }
}

pub struct PrivateFrame {
    header: FrameHeader,
    owner: String,
    data: Vec<u8>,
}

impl PrivateFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags(b"PRIV", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        PrivateFrame {
            header,
            owner: String::new(),
            data: Vec::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let owner = string::read_terminated(Encoding::Latin1, stream);
        let data = stream.take_rest().to_vec();

        Ok(PrivateFrame {
            header,
            owner,
            data,
        })
    }

    pub fn owner(&self) -> &String {
        &self.owner
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn owner_mut(&mut self) -> &mut String {
        &mut self.owner
    }

    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }
}

impl Frame for PrivateFrame {
    fn key(&self) -> String {
        format!["PRIV:{}", self.owner]
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend(string::render_terminated(Encoding::Latin1, &self.owner));
        result.extend(self.data.clone());

        result
    }
}

impl Display for PrivateFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.owner]
    }
}

impl Default for PrivateFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}

pub struct FileIdFrame {
    header: FrameHeader,
    owner: String,
    identifier: Vec<u8>,
}

impl FileIdFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags(b"UFID", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        FileIdFrame {
            header,
            owner: String::new(),
            identifier: Vec::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let owner = string::read_terminated(Encoding::Latin1, stream);
        let identifier = stream.take_rest().to_vec();

        Ok(FileIdFrame {
            header,
            owner,
            identifier,
        })
    }

    pub fn owner(&self) -> &String {
        &self.owner
    }

    pub fn identifier(&self) -> &Vec<u8> {
        &self.identifier
    }

    pub fn owner_mut(&mut self) -> &mut String {
        &mut self.owner
    }

    pub fn identifier_mut(&mut self) -> &mut Vec<u8> {
        &mut self.identifier
    }
}

impl Frame for FileIdFrame {
    fn key(&self) -> String {
        format!["UFID:{}", self.owner]
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.owner.is_empty() || self.identifier.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend(string::render_terminated(Encoding::Latin1, &self.owner));

        // Technically there can be only 64 bytes of identifier data, but nobody enforces this.
        result.extend(self.identifier.clone());

        result
    }
}

impl Display for FileIdFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.owner]
    }
}

impl Default for FileIdFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIV_DATA: &[u8] = b"test@test.com\0\
                               \x16\x16\x16\x16\x16\x16";

    const UFID_DATA: &[u8] = b"http://www.id3.org/dummy/ufid.html\0\
                               \x16\x16\x16\x16\x16\x16";

    const PRIV_EMAIL: &str = "test@test.com";
    const UFID_LINK: &str = "http://www.id3.org/dummy/ufid.html";
    const DATA: &[u8] = b"\x16\x16\x16\x16\x16\x16";

    #[test]
    fn parse_priv() {
        let frame =
            PrivateFrame::parse(FrameHeader::new(b"PRIV"), &mut BufStream::new(PRIV_DATA)).unwrap();

        assert_eq!(frame.owner(), PRIV_EMAIL);
        assert_eq!(frame.data(), DATA);
    }

    #[test]
    fn parse_ufid() {
        let frame =
            FileIdFrame::parse(FrameHeader::new(b"UFID"), &mut BufStream::new(UFID_DATA)).unwrap();

        assert_eq!(frame.owner(), UFID_LINK);
        assert_eq!(frame.identifier(), DATA);
    }

    #[test]
    fn render_priv() {
        let mut frame = PrivateFrame::new();
        frame.owner_mut().push_str(PRIV_EMAIL);
        frame.data_mut().extend(DATA);

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), PRIV_DATA);
    }

    #[test]
    fn render_ufid() {
        let mut frame = FileIdFrame::new();
        frame.owner_mut().push_str(UFID_LINK);
        frame.identifier_mut().extend(DATA);

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), UFID_DATA);
    }
}
