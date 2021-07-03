use crate::core::io::BufStream;
use crate::id3v2::frames::{Frame, FrameHeader, FrameId, Token};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct PopularimeterFrame {
    header: FrameHeader,
    pub email: String,
    pub rating: u8,
    pub plays: u64,
}

impl PopularimeterFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let email = string::read_terminated(Encoding::Latin1, stream);
        let rating = stream.read_u8()?;
        let plays = read_play_count(stream);

        Ok(Self {
            header,
            email,
            rating,
            plays,
        })
    }

    pub fn rating_simple(&self) -> u8 {
        match self.rating {
            0 => 0,
            1..=63 => 1,
            64..=127 => 2,
            128..=195 => 3,
            196..=254 => 4,
            255 => 5,
        }
    }
}

impl Frame for PopularimeterFrame {
    fn key(&self) -> String {
        format!["POPM:{}", self.email]
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        false // Can never be empty
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend(string::render_terminated(Encoding::Latin1, &self.email));
        result.push(self.rating);

        // Save some space by omitting the play count if zero
        if self.plays > 0 {
            result.extend(render_play_count(self.plays));
        }

        result
    }
}

impl Display for PopularimeterFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![
            f,
            "{}/255 [{}, plays={}]",
            self.rating, self.email, self.plays
        ]
    }
}

impl Default for PopularimeterFrame {
    fn default() -> Self {
        Self {
            header: FrameHeader::new(FrameId::new(b"POPM")),
            email: String::new(),
            plays: 0,
            rating: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlayCounterFrame {
    header: FrameHeader,
    pub plays: u64,
}

impl PlayCounterFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let plays = read_play_count(stream);

        Ok(Self { header, plays })
    }
}

impl Frame for PlayCounterFrame {
    fn key(&self) -> String {
        String::from("PCNT")
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        false // Can never be empty, even with zero plays.
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        render_play_count(self.plays)
    }
}

impl Display for PlayCounterFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.plays]
    }
}

impl Default for PlayCounterFrame {
    fn default() -> Self {
        Self {
            header: FrameHeader::new(FrameId::new(b"PCNT")),
            plays: 0,
        }
    }
}

fn read_play_count(stream: &mut BufStream) -> u64 {
    // The ID3v2 spec is frustratingly vague about how big a play counter can be,
    // so we just cap it to a u64. Should be plenty.

    match stream.read_u64() {
        Ok(plays) => plays,
        Err(_) => {
            // That didn't work. Instead try to fill in a play count lossily, leaving
            // zeroes that couldn't be filled.
            let mut arr = [0; 8];
            stream.read(&mut arr[stream.remaining()..]);
            u64::from_be_bytes(arr)
        }
    }
}

fn render_play_count(play_count: u64) -> Vec<u8> {
    let bytes = play_count.to_be_bytes();

    for i in 0..4 {
        // The size is larger than a 4-bytes, so return the first four bytes
        // plus the populated byte we just found.
        if bytes[i] > 0 {
            return bytes[i..].into();
        }
    }

    // Otherwise return the first four bytes, the hard-limit by the spec.
    bytes[4..].into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::tag::Version;

    const POPM_DATA: &[u8] = b"test@test.com\0\
                               \x80\
                               \x00\x00\x16\x16";

    const PCNT_DATA: &[u8] = b"\x00\x00\x16\x16";

    #[test]
    fn parse_popm() {
        let frame = PopularimeterFrame::parse(
            FrameHeader::new(FrameId::new(b"POPM")),
            &mut BufStream::new(POPM_DATA),
        )
        .unwrap();

        assert_eq!(frame.email, "test@test.com");
        assert_eq!(frame.rating, 0x80);
        assert_eq!(frame.plays, 0x1616);
    }

    #[test]
    fn parse_pcnt() {
        let frame = PlayCounterFrame::parse(
            FrameHeader::new(FrameId::new(b"PCNT")),
            &mut BufStream::new(PCNT_DATA),
        )
        .unwrap();

        assert_eq!(frame.plays, 0x1616)
    }

    #[test]
    fn render_popm() {
        let mut frame = PopularimeterFrame::new();
        frame.email.push_str("test@test.com");
        frame.rating = 0x80;
        frame.plays = 0x1616;

        assert!(!frame.is_empty());
        assert_eq!(
            frame.render(&TagHeader::with_version(Version::V24)),
            POPM_DATA
        );
    }

    #[test]
    fn render_pcnt() {
        let mut frame = PlayCounterFrame::new();
        frame.plays = 0x1616;

        assert!(!frame.is_empty());
        assert_eq!(
            frame.render(&TagHeader::with_version(Version::V24)),
            PCNT_DATA
        );
    }

    #[test]
    fn render_large_play_counts() {
        let plays: u64 = 0x123456789ABCD;

        assert_eq!(render_play_count(plays), b"\x01\x23\x45\x67\x89\xAB\xCD");
    }
}
