//! Media statistics frames.

use crate::core::io::BufStream;
use crate::id3v2::frames::{Frame, FrameId};
use crate::id3v2::{ParseResult, TagHeader};
use crate::core::string::{self, Encoding};
use log::info;
use std::fmt::{self, Display, Formatter};

#[derive(Default, Debug, Clone)]
pub struct PopularimeterFrame {
    pub email: String,
    pub rating: u8,
    pub plays: u64,
}

impl PopularimeterFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let email = string::read_terminated(Encoding::Latin1, stream);
        let rating = stream.read_u8()?;
        let plays = read_play_count(stream);

        Ok(Self {
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
    fn id(&self) -> FrameId {
        FrameId::new(b"POPM")
    }

    fn key(&self) -> String {
        format!["POPM:{}", self.email]
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
            info!("omitting play count of 0");
            result.extend(render_play_count(self.plays));
        }

        result
    }
}

impl Display for PopularimeterFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![
            f,
            "{} [{}/255, plays={}]",
            self.email, self.rating, self.plays
        ]
    }
}

#[derive(Default, Debug, Clone)]
pub struct PlayCounterFrame {
    pub plays: u64,
}

impl PlayCounterFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let plays = read_play_count(stream);

        Ok(Self { plays })
    }
}

impl Frame for PlayCounterFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"PCNT")
    }

    fn key(&self) -> String {
        String::from("PCNT")
    }

    fn is_empty(&self) -> bool {
        // This frame is never empty, even with zero plays.
        false
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

    const POPM_DATA: &[u8] = b"POPM\x00\x00\x00\x13\x00\x00\
                               test@test.com\0\
                               \x80\
                               \x00\x00\x16\x16";

    const PCNT_DATA: &[u8] = b"PCNT\x00\x00\x00\x04\x00\x00\
                               \x00\x00\x16\x16";

    #[test]
    fn parse_popm() {
        make_frame!(PopularimeterFrame, POPM_DATA, frame);

        assert_eq!(frame.email, "test@test.com");
        assert_eq!(frame.rating, 0x80);
        assert_eq!(frame.plays, 0x1616);
    }

    #[test]
    fn parse_pcnt() {
        make_frame!(PlayCounterFrame, PCNT_DATA, frame);

        assert_eq!(frame.plays, 0x1616)
    }

    #[test]
    fn render_popm() {
        let frame = PopularimeterFrame {
            email: String::from("test@test.com"),
            rating: 0x80,
            plays: 0x1616,
        };

        assert_render!(frame, POPM_DATA);
    }

    #[test]
    fn render_pcnt() {
        let frame = PlayCounterFrame { plays: 0x1616 };

        assert_render!(frame, PCNT_DATA);
    }

    #[test]
    fn render_large_play_counts() {
        let plays: u64 = 0x123456789ABCD;

        assert_eq!(render_play_count(plays), b"\x01\x23\x45\x67\x89\xAB\xCD");
    }
}
