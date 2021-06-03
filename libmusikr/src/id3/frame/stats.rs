use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};
use crate::raw;
use std::fmt::{self, Display, Formatter};

pub struct PopularimeterFrame {
    header: Id3FrameHeader,
    email: String,
    rating: u8,
    plays: u32
}

impl PopularimeterFrame {
    pub(super) fn new(header: Id3FrameHeader, data: &[u8]) -> PopularimeterFrame {
        let (email, email_size) = string::get_terminated_string(Encoding::Utf8, data);
        let rating = *data.get(email_size).unwrap_or(&0);

        let mut play_data = &data[email_size + 1..];

        // Technically, play counts can be infinite in size, but we cap it to a u32 for simplicity. 
        if play_data.len() > 4 {
            play_data = &play_data[..play_data.len() - 4];
        }

        let plays = raw::to_u32(play_data);

        PopularimeterFrame {
            header,
            email,
            rating,
            plays
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
            255 => 5  
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
}

impl Display for PopularimeterFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "[{}] rating={}, plays={}", self.email, self.rating, self.plays]
    }
}