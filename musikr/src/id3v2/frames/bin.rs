use crate::core::io::BufStream;
use crate::id3v2::frames::{Frame, FrameHeader, Token};
use crate::id3v2::{ParseError, ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

pub struct UnknownFrame {
    header: FrameHeader,
    data: Box<[u8]>,
}

impl UnknownFrame {
    pub(crate) fn from_stream(header: FrameHeader, stream: &mut BufStream) -> Self {
        Self {
            header,
            data: stream.take_rest().to_vec().into_boxed_slice(),
        }
    }

    pub fn data(&self) -> &[u8] {
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
        self.data.to_vec()
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

pub struct FileIdFrame {
    header: FrameHeader,
    pub owner: String,
    pub identifier: Vec<u8>,
}

impl FileIdFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let owner = string::read_terminated(Encoding::Latin1, stream);
        let identifier = stream.take_rest().to_vec();

        Ok(Self {
            header,
            owner,
            identifier,
        })
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
        result.extend(self.identifier.iter());

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
        Self {
            header: FrameHeader::new(b"UFID"),
            owner: String::new(),
            identifier: Vec::new()
        }
    }
}

pub struct PrivateFrame {
    header: FrameHeader,
    pub owner: String,
    pub data: Vec<u8>,
}

impl PrivateFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let owner = string::read_terminated(Encoding::Latin1, stream);
        let data = stream.take_rest().to_vec();

        Ok(Self {
            header,
            owner,
            data,
        })
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
        Self {
            header: FrameHeader::new(b"PRIV"),
            owner: String::new(),
            data: Vec::new()
        }
    }
}

pub struct PodcastFrame {
    header: FrameHeader,
}

impl PodcastFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        // The iTunes podcast frame is for some reason just four zeroes that flag this file as
        // being a podcast, meaning that this frames existence is pretty much the only form of
        // mutability it has. Just validate the given data and move on.
        if stream.take_rest() != b"\0\0\0\0" {
            return Err(ParseError::MalformedData);
        }

        Ok(PodcastFrame { header })
    }
}

impl Frame for PodcastFrame {
    fn key(&self) -> String {
        String::from("PCST")
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        // Frame is a constant 4 bytes, so it is never empty
        false
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        vec![0x00, 0x00, 0x00, 0x00]
    }
}

impl Display for PodcastFrame {
    fn fmt(&self, _f: &mut Formatter) -> fmt::Result {
        // Nothing to format.
        Ok(())
    }
}

impl Default for PodcastFrame {
    fn default() -> Self {
        Self {
            header: FrameHeader::new(b"PCST")
        }
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

    const PCST_DATA: &[u8] = b"\0\0\0\0";

    #[test]
    fn parse_priv() {
        let frame =
            PrivateFrame::parse(FrameHeader::new(b"PRIV"), &mut BufStream::new(PRIV_DATA)).unwrap();

        assert_eq!(frame.owner, PRIV_EMAIL);
        assert_eq!(frame.data, DATA);
    }

    #[test]
    fn parse_ufid() {
        let frame =
            FileIdFrame::parse(FrameHeader::new(b"UFID"), &mut BufStream::new(UFID_DATA)).unwrap();

        assert_eq!(frame.owner, UFID_LINK);
        assert_eq!(frame.identifier, DATA);
    }

    #[test]
    fn render_priv() {
        let mut frame = PrivateFrame::new();
        frame.owner.push_str(PRIV_EMAIL);
        frame.data.extend(DATA);

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), PRIV_DATA);
    }

    #[test]
    fn render_ufid() {
        let mut frame = FileIdFrame::new();
        frame.owner.push_str(UFID_LINK);
        frame.identifier.extend(DATA);

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), UFID_DATA);
    }

    #[test]
    fn parse_pcst() {
        PodcastFrame::parse(FrameHeader::new(b"PCST"), &mut BufStream::new(PCST_DATA)).unwrap();
    }

    #[test]
    fn render_pcst() {
        assert_eq!(
            PodcastFrame::new().render(&TagHeader::with_version(4)),
            PCST_DATA
        )
    }
}
