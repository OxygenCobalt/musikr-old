//! ID3v2 tag reading/writing.
//!
//! ID3v2 is the most common tag format, being the primary tag format in MP3 files and
//! having a presence in other files as well. However, its also one of the most complex
//! tag formats, making this module one of the less ergonomic and more complicated APIs
//! to use in musikr.
//!
//! # Tag structure
//!
//! ID3v2 tags are represented by the [`Tag`](Tag) struct and are largely structured as the following:
//!
//! ```text
//! ID3 [Version] [Flags] [Size]
//! [Optional Extended Header]
//! [Frame Data]
//! [Footer OR Padding, can be absent]
//! ```
//!
//! ID3v2 tags will always start with a header [Represented by [`TagHeader`](crate::id3v2::tag::TagHeader)]
//! that contains an identifier, a version, and the total tag size.
//!
//! This is then optionally followed by an extended header [Represented by [`ExtendedHeader`](crate::id3v2::tag::ExtendedHeader)]
//! that contains optional information about the tag.
//!
//! The frame data usually follows, which contains the actual audio metadata. More information can be found
//! about frames in [`id3v2::frames`](crate::id3v2::frames).
//!
//! The tag is then either ended with a Footer [Effectively a clone of [`TagHeader`](crate::id3v2::tag::TagHeader)],
//! a group of zeroes for padding, or nothing.
//!
//! For more information, see the individual items linked or read the [ID3v2 specification](http://id3.org/id3v2.4.0-structure).

pub mod frame_map;
pub mod frames;
mod syncdata;
pub mod tag;

use crate::core::io::BufStream;
use frame_map::FrameMap;
use tag::{ExtendedHeader, HeaderResult, TagHeader, Version};

use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

// TODO: The current roadmap for this module:
// - Try to complete most if not all of the frame specs
// - Add further documentation
// - Work on tag upgrading
// - Add proper tag writing
// - Logging?

#[derive(Debug, Clone)]
pub struct Tag {
    header: TagHeader,
    pub extended_header: Option<ExtendedHeader>,
    pub frames: FrameMap,
}

impl Tag {
    pub fn new(version: Version) -> Self {
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

        let mut header = match TagHeader::parse(header_raw) {
            HeaderResult::Ok(header) => header,
            // TODO: ID3v2.2 upgrading.
            HeaderResult::Version22 => return Err(ParseError::Unsupported),
            HeaderResult::Err(err) => return Err(err),
        };

        // Then get the full tag data. If the size is invalid, then we will just truncate it.
        let mut tag_data = vec![0; header.size()];
        let read = file.read(&mut tag_data)?;
        tag_data.truncate(read);

        // Begin body parsing. This is where the data becomes a stream instead of a vector.
        let mut stream = BufStream::new(&tag_data);

        let (extended_header, frames) = {
            if header.version() == Version::V23 && header.flags().unsync {
                // ID3v2.3 tag-specific unsynchronization, decode the stream here.
                parse_body(&mut header, BufStream::new(&syncdata::decode(&mut stream)))
            } else {
                parse_body(&mut header, stream)
            }
        };

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

fn parse_body(
    tag_header: &mut TagHeader,
    mut stream: BufStream,
) -> (Option<ExtendedHeader>, FrameMap) {
    let mut ext_header = None;

    if tag_header.flags().extended {
        // Certain taggers will flip the extended header flag without writing one,
        // so if parsing fails then we correct the flag.
        match ExtendedHeader::parse(&mut stream, tag_header.version()) {
            Ok(header) => ext_header = Some(header),
            Err(_) => tag_header.flags_mut().extended = false,
        }
    }

    // Now try parsing our frames,
    let mut frames = FrameMap::new();

    while let Ok(frame) = frames::new(&tag_header, &mut stream) {
        frames.add(frame);
    }

    (ext_header, frames)
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
