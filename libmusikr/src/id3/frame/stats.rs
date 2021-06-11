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
    pub fn new(header: FrameHeader) -> Self {
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

    fn parse(&mut self, data: &[u8]) -> Result<(), ()> {
        if data.len() < 2 {
            return Err(()); // Not enough data
        }

        let email = string::get_terminated_string(Encoding::Utf8, data);
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

pub struct PlayCounterFrame {
    header: FrameHeader,
    plays: u32,
}

impl PlayCounterFrame {
    pub fn new(header: FrameHeader) -> Self {
        PlayCounterFrame { header, plays: 0 }
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

    fn parse(&mut self, data: &[u8]) -> Result<(), ()> {
        if data.len() < 4 {
            return Err(()); // Not enough data
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
