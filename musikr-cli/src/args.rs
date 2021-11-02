use std::error;
use std::fmt::{self, Display, Formatter};
use std::io;

static TAG_NAMES: &[&str] = &["album", "artist", "comment", "date", "genre", "title", "track"];
static ID3V2_ANALOGUES: &[&[u8; 4]] = &[b"TALB", b"TPE1", b"COMM", b"TDRC", b"TCON", b"TIT2", b"TRCK"];

// TODO: Add more ID3v2 analogues.

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ReadTag {
    Album = 0,
    Artist = 1,
    Comment = 2,
    Date = 3,
    Title = 4,
    Track = 5,
    Genre = 6
}

impl ReadTag {
    pub fn from_arg(arg: &str) -> Result<Self, OpError> {
        let tag = match arg {
            "album" => Self::Album,
            "artist" => Self::Artist,
            "comment" => Self::Comment,
            "date" => Self::Date,
            "genre" => Self::Genre,
            "title" => Self::Title,
            "track" => Self::Track,
            _ => return Err(OpError::InvalidTag(arg.to_string())),
        };

        Ok(tag)
    }

    pub fn from_id3v2(frame_id: musikr::id3v2::frames::FrameId) -> Option<Self> {
        let tag = match frame_id.inner() {
            b"TALB" => Self::Album,
            b"TPE1" => Self::Artist,
            b"TCOM" => Self::Comment,
            b"TDRC" => Self::Date,
            b"TCON" => Self::Genre,
            b"TIT2" => Self::Title,
            b"TRCK" => Self::Track,
            _ => return None
        };

        Some(tag)
    }

    pub fn as_id3v2(&self) -> &[u8; 4] {
        &ID3V2_ANALOGUES[*self as usize]
    }
}

impl Display for ReadTag {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write![f, "{}", TAG_NAMES[*self as usize]]
    }
}

#[derive(Debug)]
pub enum OpError {
    IoError(io::Error),
    InvalidTag(String),
    MalformedMetadata,
    UnsupportedMetadata,
}

impl Display for OpError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::IoError(err) => write![f, "{}", err],
            Self::InvalidTag(tag) => write![f, "Found invalid tag \"{}\"", tag],
            Self::MalformedMetadata => write![f, "Malformed metadata"],
            Self::UnsupportedMetadata => write![f, "Unsupported metadata"],
        }
    }
}

impl error::Error for OpError {
    // Nothing to implement
}

impl From<io::Error> for OpError {
    fn from(other: io::Error) -> Self {
        return Self::IoError(other);
    }
}
