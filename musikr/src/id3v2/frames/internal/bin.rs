use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::{ParseError, TagHeader};
use std::fmt::{self, Display, Formatter};

pub struct RawFrame {
    header: FrameHeader,
    data: Vec<u8>,
}

impl RawFrame {
    pub fn new(frame_id: &str) -> Self {
        Self::with_flags(frame_id, FrameFlags::default())
    }

    pub fn with_flags(frame_id: &str, flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags(frame_id, flags))
    }

    pub(crate) fn with_data(header: FrameHeader, data: &[u8]) -> Self {
        RawFrame {
            header,
            data: data.to_vec(),
        }
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        RawFrame {
            header,
            data: Vec::new(),
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }
}

impl Frame for RawFrame {
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        self.id().clone()
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        self.data.clone()
    }
}

impl Display for RawFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt_vec_hexstream(f, &self.data)
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
        Self::with_header(FrameHeader::with_flags("PRIV", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        PrivateFrame {
            header,
            owner: String::new(),
            data: Vec::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < 2 {
            // A private frame must have at least an empty owner string and 1 byte of data
            return Err(ParseError::NotEnoughData);
        }

        let owner = string::get_terminated(Encoding::Latin1, data);
        let data = data[owner.size..].to_vec();

        Ok(PrivateFrame {
            header,
            owner: owner.string,
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
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        format!["{}:{}", self.id(), self.owner]
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
        Self::with_header(FrameHeader::with_flags("UFID", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        FileIdFrame {
            header,
            owner: String::new(),
            identifier: Vec::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < 3 {
            // A UFID frame must have a non-empty owner string and 1 byte of identifier data
            return Err(ParseError::NotEnoughData);
        }

        let owner = string::get_terminated(Encoding::Latin1, data);
        let identifier = data[owner.size..].to_vec();

        Ok(FileIdFrame {
            header,
            owner: owner.string,
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
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        format!["{}:{}", self.id(), self.owner]
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

fn fmt_vec_hexstream(f: &mut Formatter, vec: &[u8]) -> fmt::Result {
    let data = if vec.len() > 64 {
        // Truncate the hex data to 64 bytes
        &vec[0..64]
    } else {
        vec
    };

    for byte in data {
        write![f, "{:02x}", byte]?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_priv() {
        let data = b"test@test.com\0\
                     \x16\x16\x16\x16\x16\x16";

        let frame = PrivateFrame::parse(FrameHeader::new("PRIV"), &data[..]).unwrap();

        assert_eq!(frame.owner(), "test@test.com");
        assert_eq!(frame.data(), b"\x16\x16\x16\x16\x16\x16")
    }

    #[test]
    fn parse_ufid() {
        let data = b"http://www.id3.org/dummy/ufid.html\0\
                     \x16\x16\x16\x16\x16\x16";

        let frame = FileIdFrame::parse(FrameHeader::new("UFID"), &data[..]).unwrap();

        assert_eq!(frame.owner(), "http://www.id3.org/dummy/ufid.html");
        assert_eq!(frame.identifier(), b"\x16\x16\x16\x16\x16\x16")
    }

    #[test]
    fn render_priv() {
        let out = b"test@test.com\0\
                    \x16\x16\x16\x16\x16\x16";

        let mut frame = PrivateFrame::new();
        frame.owner_mut().push_str("test@test.com");
        frame.data_mut().extend(b"\x16\x16\x16\x16\x16\x16");

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), out);
    }

    #[test]
    fn render_ufid() {
        let out = b"http://www.id3.org/dummy/ufid.html\0\
                    \x16\x16\x16\x16\x16\x16";

        let mut frame = FileIdFrame::new();
        frame
            .owner_mut()
            .push_str("http://www.id3.org/dummy/ufid.html");
        frame.identifier_mut().extend(b"\x16\x16\x16\x16\x16\x16");

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), out);
    }
}
