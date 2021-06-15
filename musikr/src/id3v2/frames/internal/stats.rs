use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::{ParseError, TagHeader};
use crate::raw;
use std::fmt::{self, Display, Formatter};

pub struct PopularimeterFrame {
    header: FrameHeader,
    email: String,
    rating: u8,
    plays: u32,
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

    pub fn email(&self) -> &String {
        &self.email
    }

    pub fn rating(&self) -> u8 {
        self.rating
    }

    pub fn plays(&self) -> u32 {
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

    fn parse(&mut self, _header: &TagHeader, data: &[u8]) -> Result<(), ParseError> {
        if data.len() < 2 {
            return Err(ParseError::NotEnoughData); // Not enough data
        }

        let email = string::get_terminated_string(Encoding::Latin1, data);
        self.email = email.string;
        self.rating = data[email.size];

        if data.len() > email.size {
            let mut play_data = &data[email.size + 1..];

            // Technically, play counts can be infinite in size, but we cap it to a u32 for simplicity.
            if play_data.len() > 4 {
                play_data = &play_data[..play_data.len() - 4];
            }

            self.plays = raw::to_u32(play_data);
        }

        Ok(())
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
    plays: u32,
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

    pub fn plays(&self) -> u32 {
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

    fn parse(&mut self, _header: &TagHeader, data: &[u8]) -> Result<(), ParseError> {
        if data.len() < 4 {
            return Err(ParseError::NotEnoughData);
        }

        self.plays = raw::to_u32(data);

        Ok(())
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
