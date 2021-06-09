use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{FrameHeader, Id3Frame};
use crate::raw;
use std::fmt::{self, Display, Formatter};

pub struct PopularimeterFrame {
    header: FrameHeader,
    email: String,
    rating: u8,
    plays: u32,
}

impl PopularimeterFrame {
    pub(crate) fn new(header: FrameHeader, data: &[u8]) -> Option<Self> {
        if data.len() < 6 {
            return None;
        }

        let (email, email_size) = string::get_terminated_string(Encoding::Utf8, data);
        let rating = *data.get(email_size).unwrap_or(&0);

        let mut play_data = &data[email_size + 1..];

        // Technically, play counts can be infinite in size, but we cap it to a u32 for simplicity.
        if play_data.len() > 4 {
            play_data = &play_data[..play_data.len() - 4];
        }

        let plays = raw::to_u32(play_data);

        Some(PopularimeterFrame {
            header,
            email,
            rating,
            plays,
        })
    }

    pub fn from(frame: Box<dyn Id3Frame>) -> Option<Box<Self>> {
        downcast!(frame, Self)
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

impl Id3Frame for PopularimeterFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
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

pub struct PlayCounterFrame {
    header: FrameHeader,
    plays: u32,
}

impl PlayCounterFrame {
    pub(crate) fn new(header: FrameHeader, data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }

        let plays = raw::to_u32(data);

        Some(PlayCounterFrame { header, plays })
    }

    pub fn plays(&self) -> u32 {
        self.plays
    }
}

impl Id3Frame for PlayCounterFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
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
