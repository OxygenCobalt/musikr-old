//! ID3v2 tag reading/writing.
//!
//! ID3v2 is the most common tag format, being the primary tag format in MP3 files and
//! having a presence in other files as well. However, its also one of the most complex
//! tag formats, making this module one of the less ergonomic and more complicated APIs
//! to use in musikr.
//!
//! The ID3v2 module assumes that you have working knowledge of the ID3v2 tag format, so
//! it's reccomended to read the [ID3v2.3](https://id3.org/id3v2.3.0) and
//! [ID3v2.4](https://id3.org/id3v2.4.0-structure) documents to get a better idea of the
//! tag structure.
//!
//! # Usage

pub mod frame_map;
pub mod frames;
mod syncdata;
pub mod tag;

use crate::core::io::BufStream;
use frame_map::FrameMap;
use tag::{ExtendedHeader, TagHeader, Version};

use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

// TODO: The current roadmap for this module:
// - Logging [with my own macros]
// - Toss unknown frames into their own map
// - Drop empty frames
// - Try to complete most if not all of the frame specs
// - Add further documentation
// - Work on tag upgrading
// - Add proper tag writing

#[derive(Debug, Clone)]
pub struct Tag {
    header: TagHeader,
    pub extended_header: Option<ExtendedHeader>,
    pub frames: FrameMap,
}

impl Tag {
    pub fn new(version: Version) -> Self {
        if version == Version::V22 {
            panic!("ID3v2.2 tags cannot be created, only upgraded from a file.")
        }

        Tag {
            header: TagHeader::with_version(version),
            extended_header: None,
            frames: FrameMap::new(),
        }
    }

    pub fn open<P: AsRef<Path>>(path: P) -> ParseResult<Self> {
        let mut file = File::open(path)?;

        // Read and parse the possible ID3v2 header
        let mut header_raw = [0; 10];
        file.read_exact(&mut header_raw)?;

        let mut header = TagHeader::parse(header_raw).map_err(|err| match err {
            ParseError::MalformedData => ParseError::NotFound,
            err => err,
        })?;

        // Then get the full tag data. If the size is invalid, then we will just truncate it.
        let mut tag_data = vec![0; header.size() as usize];
        let read = file.read(&mut tag_data)?;
        tag_data.truncate(read);

        let mut stream = BufStream::new(&tag_data);

        // ID3v2.3 tag-specific unsynchronization, decode the stream here.
        if header.version() < Version::V24 && header.flags().unsync {
            tag_data = syncdata::decode(&mut stream);
            stream = BufStream::new(&tag_data);
        }

        let mut extended_header = None;

        if header.flags().extended {
            // Certain taggers will flip the extended header flag without writing one,
            // so if parsing fails then we correct the flag.
            match ExtendedHeader::parse(&mut stream, header.version()) {
                Ok(header) => extended_header = Some(header),
                Err(_) => header.flags_mut().extended = false,
            }
        }

        // Now try parsing our frames.
        let mut frames = FrameMap::new();

        while let Ok(frame) = frames::parse(&header, &mut stream) {
            frames.add(frame);
        }

        Ok(Self {
            header,
            extended_header,
            frames,
        })
    }

    /// Returns an immutable reference to the header of this tag.
    pub fn header(&self) -> &TagHeader {
        &self.header
    }

    pub fn version(&self) -> Version {
        self.header.version()
    }
}

impl Default for Tag {
    fn default() -> Self {
        Self::new(Version::V24)
    }
}

/// The result given after a parsing operation.
pub type ParseResult<T> = Result<T, ParseError>;

/// The error type returned when parsing ID3v2 tags.
#[derive(Debug)]
pub enum ParseError {
    /// Generic IO errors. This either means that a problem occured while opening the file
    /// for a tag, or an unexpected EOF was encounted while parsing.
    IoError(io::Error),
    /// A part of the tag was not valid.
    MalformedData,
    /// The tag or a component of the tag is unsupported by musikr.
    Unsupported,
    /// The tag was not found in the given file.
    NotFound,
}

impl From<io::Error> for ParseError {
    fn from(other: io::Error) -> Self {
        ParseError::IoError(other)
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for ParseError {
    // Nothing to implement
}

/// The result given after a parsing operation.
pub type SaveResult<T> = Result<T, SaveError>;

/// The error type returned when parsing ID3v2 tags.
#[derive(Debug)]
pub enum SaveError {
    /// Generic IO errors. This means that a problem occured while writing.
    IoError(io::Error),
    /// The tag or an element in the tag ended up being too large.
    TooLarge,
}

impl From<io::Error> for SaveError {
    fn from(other: io::Error) -> Self {
        SaveError::IoError(other)
    }
}

impl Display for SaveError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for SaveError {
    // Nothing to implement
}
