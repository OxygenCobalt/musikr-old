use std::io;

pub static TAG_NAMES: &[&str] = &["album", "artist", "genre", "title", "track", "date"];

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ReadTag {
	Album  = 0,
	Artist = 1,
	Genre  = 2,
	Title  = 3,
	Track  = 4,
	Date   = 5,
}

impl ReadTag {
	pub(crate) fn from_arg(arg: &str) -> Option<Self> {
		let tag = match arg {
			"album" => Self::Album,
			"artist" => Self::Artist,
			"genre" => Self::Genre,
			"title" => Self::Title,
			"track" => Self::Track,
			"date" => Self::Date,
			_ => return None
		};

		Some(tag)
    }

    pub fn name(&self) -> &str {
    	&TAG_NAMES[*self as usize]
    }
}

pub enum OpError {
	IoError(io::Error),
	Invalid,
	Unsupported,
}

