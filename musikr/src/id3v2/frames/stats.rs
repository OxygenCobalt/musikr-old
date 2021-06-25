use crate::core::io::BufStream;
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader, Token};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

pub struct PopularimeterFrame {
    header: FrameHeader,
    email: String,
    rating: u8,
    plays: u64,
}

impl PopularimeterFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags(b"POPM", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        PopularimeterFrame {
            header,
            email: String::new(),
            rating: 0,
            plays: 0,
        }
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let email = string::read_terminated(Encoding::Latin1, stream);
        let rating = stream.read_u8()?;
        let plays = read_play_count(stream);

        Ok(PopularimeterFrame {
            header,
            email,
            rating,
            plays,
        })
    }

    pub fn email(&self) -> &String {
        &self.email
    }

    pub fn rating(&self) -> u8 {
        self.rating
    }

    pub fn plays(&self) -> u64 {
        self.plays
    }

    pub fn email_mut(&mut self) -> &mut String {
        &mut self.email
    }

    pub fn rating_mut(&mut self) -> &mut u8 {
        &mut self.rating
    }

    pub fn plays_mut(&mut self) -> &mut u64 {
        &mut self.plays
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
        Self::with_flags(FrameFlags::default())
    }
}

pub struct PlayCounterFrame {
    header: FrameHeader,
    plays: u64,
}

impl PlayCounterFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags(b"PCNT", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        PlayCounterFrame { header, plays: 0 }
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let plays = read_play_count(stream);

        Ok(PlayCounterFrame { header, plays })
    }

    pub fn plays(&self) -> u64 {
        self.plays
    }

    pub fn plays_mut(&mut self) -> &mut u64 {
        &mut self.plays
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
        Self::with_flags(FrameFlags::default())
    }
}

fn read_play_count(stream: &mut BufStream) -> u64 {
    // The ID3v2 spec is frustratingly vague about how big a play counter can be,
    // so we just cap it to a u64. Should be plenty.

    match stream.read_u64() {
        Ok(plays) => plays,
        Err(_) => {
            // That didn't work. We need to then instead to fill an array, leaving zeroes
            // where it couldn't be filled. This is done in reverse since ID3v2 specifies that
            // these slices must be in big-endian order.
            let mut arr = [0; 8];

            for byte in arr[stream.remaining()..].iter_mut() {
                *byte = stream.read_u8().unwrap()
            }

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

    const POPM_DATA: &[u8] = b"test@test.com\0\
                               \x80\
                               \x00\x00\x16\x16";

    const PCNT_DATA: &[u8] = b"\x00\x00\x16\x16";

    #[test]
    fn parse_popm() {
        let frame =
            PopularimeterFrame::parse(FrameHeader::new(b"POPM"), &mut BufStream::new(POPM_DATA))
                .unwrap();

        assert_eq!(frame.email(), "test@test.com");
        assert_eq!(frame.rating(), 0x80);
        assert_eq!(frame.plays(), 0x1616);
    }

    #[test]
    fn parse_pcnt() {
        let frame =
            PlayCounterFrame::parse(FrameHeader::new(b"PCNT"), &mut BufStream::new(PCNT_DATA))
                .unwrap();

        assert_eq!(frame.plays(), 0x1616)
    }

    #[test]
    fn render_popm() {
        let mut frame = PopularimeterFrame::new();
        frame.email_mut().push_str("test@test.com");
        *frame.rating_mut() = 0x80;
        *frame.plays_mut() = 0x1616;

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), POPM_DATA);
    }

    #[test]
    fn render_pcnt() {
        let mut frame = PlayCounterFrame::new();
        *frame.plays_mut() = 0x1616;

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), PCNT_DATA);
    }

    #[test]
    fn render_large_play_counts() {
        let plays: u64 = 0x123456789ABCD;

        assert_eq!(render_play_count(plays), b"\x01\x23\x45\x67\x89\xAB\xCD");
    }
}
