use crate::core::io::BufStream;
use crate::id3v2::frames::{Frame, FrameId};
use crate::id3v2::{ParseError, ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct UnknownFrame {
    frame_id: FrameId,
    data: Box<[u8]>,
}

impl UnknownFrame {
    pub(crate) fn from_stream(frame_id: FrameId, stream: &mut BufStream) -> Self {
        Self {
            frame_id,
            data: stream.take_rest().to_vec().into_boxed_slice(),
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl Frame for UnknownFrame {
    fn id(&self) -> FrameId {
        self.frame_id
    }

    fn key(&self) -> String {
        self.id().to_string()
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

#[derive(Debug, Clone)]
pub struct FileIdFrame {
    pub owner: String,
    pub identifier: Vec<u8>,
}

impl FileIdFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let owner = string::read_terminated(Encoding::Latin1, stream);
        let identifier = stream.take_rest().to_vec();

        Ok(Self { owner, identifier })
    }
}

impl Frame for FileIdFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"UFID")
    }

    fn key(&self) -> String {
        format!["UFID:{}", self.owner]
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
            owner: String::new(),
            identifier: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrivateFrame {
    pub owner: String,
    pub data: Vec<u8>,
}

impl PrivateFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let owner = string::read_terminated(Encoding::Latin1, stream);
        let data = stream.take_rest().to_vec();

        Ok(Self { owner, data })
    }
}

impl Frame for PrivateFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"PRIV")
    }

    fn key(&self) -> String {
        format!["PRIV:{}", self.owner]
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
            owner: String::new(),
            data: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PodcastFrame;

impl PodcastFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        // The iTunes podcast frame is for some reason just four zeroes that flag this file as
        // being a podcast, meaning that this frames existence is pretty much the only form of
        // mutability it has. Just validate the given data and move on.
        if stream.take_rest() != b"\0\0\0\0" {
            return Err(ParseError::MalformedData);
        }

        Ok(PodcastFrame)
    }
}

impl Frame for PodcastFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"PCST")
    }

    fn key(&self) -> String {
        String::from("PCST")
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
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIV_DATA: &[u8] = b"PRIV\x00\x00\x00\x14\x00\x00\
                               test@test.com\0\
                               \x16\x16\x16\x16\x16\x16";

    const UFID_DATA: &[u8] = b"UFID\x00\x00\x00\x29\x00\x00\
                               http://www.id3.org/dummy/ufid.html\0\
                               \x16\x16\x16\x16\x16\x16";

    const PCST_DATA: &[u8] = b"PCST\x00\x00\x00\x04\x00\x00\
                               \0\0\0\0";

    #[test]
    fn parse_priv() {
        crate::make_frame!(PrivateFrame, PRIV_DATA, frame);

        assert_eq!(frame.owner, "test@test.com");
        assert_eq!(frame.data, b"\x16\x16\x16\x16\x16\x16");
    }

    #[test]
    fn parse_ufid() {
        crate::make_frame!(FileIdFrame, UFID_DATA, frame);

        assert_eq!(frame.owner, "http://www.id3.org/dummy/ufid.html");
        assert_eq!(frame.identifier, b"\x16\x16\x16\x16\x16\x16");
    }

    #[test]
    fn render_priv() {
        let frame = PrivateFrame {
            owner: String::from("test@test.com"),
            data: Vec::from(&b"\x16\x16\x16\x16\x16\x16"[..]),
        };

        crate::assert_render!(frame, PRIV_DATA);
    }

    #[test]
    fn render_ufid() {
        let frame = FileIdFrame {
            owner: String::from("http://www.id3.org/dummy/ufid.html"),
            identifier: Vec::from(&b"\x16\x16\x16\x16\x16\x16"[..]),
        };

        crate::assert_render!(frame, UFID_DATA);
    }

    #[test]
    fn parse_pcst() {
        crate::make_frame!(PodcastFrame, PCST_DATA, _f);
    }

    #[test]
    fn render_pcst() {
        crate::assert_render!(PodcastFrame, PCST_DATA);
    }
}
