use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::ParseError;
use crate::raw;
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
        Self::with_header(FrameHeader::with_flags("POPM", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        PopularimeterFrame {
            header,
            email: String::new(),
            rating: 0,
            plays: 0,
        }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < 2 {
            return Err(ParseError::NotEnoughData); // Not enough data
        }

        let email = string::get_terminated_string(Encoding::Latin1, data);
        let rating = data[email.size];
        let mut plays = 0;

        // Play count is optional
        if data.len() > email.size {
            // The ID3v2 spec is frustratingly vague about how big a play counter can be,
            // so we just cap it to a u64. Should be plenty.
            plays = raw::to_u64(&data[email.size + 1..]);
        }

        Ok(PopularimeterFrame {
            header,
            email: email.string,
            rating,
            plays
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
        format!["{}:{}", self.id(), self.email]
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
        Self::with_header(FrameHeader::with_flags("PCNT", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        PlayCounterFrame { header, plays: 0 }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < 4 {
            return Err(ParseError::NotEnoughData);
        }

        // The ID3v2 spec is frustratingly vague about how big a play counter can be,
        // so we just cap it to a u64. Should be plenty.
        let plays = raw::to_u64(data);

        Ok(PlayCounterFrame {
            header,
            plays
        })
    }

    pub fn plays(&self) -> u64 {
        self.plays
    }
}

impl Frame for PlayCounterFrame {
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
