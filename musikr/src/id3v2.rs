pub mod frame_map;
pub mod frames;
mod syncdata;
pub mod tag;

use crate::core::io::BufStream;
use frame_map::FrameMap;
use tag::ExtendedHeader;
use tag::TagHeader;

use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

// TODO: The current roadmap for this module:
// - Improve current frame implementation
// - Try to complete most if not all of the frame specs
// - Work on tag upgrading, improve versioning using an enum?
// - Add proper tag writing
// - Work on properly deriving certain attributes [Such as Debug, Copy, PartialEq]

#[allow(dead_code)]
pub struct Tag {
    file: Option<File>,
    offset: u64,
    header: TagHeader,
    ext_header: Option<ExtendedHeader>,
    frames: FrameMap,
}

impl Tag {
    pub fn new(version: u8) -> Self {
        Tag {
            file: None,
            offset: 0,
            header: TagHeader::with_version(version),
            ext_header: None,
            frames: FrameMap::new(),
        }
    }

    pub fn open<P: AsRef<Path>>(path: P) -> ParseResult<Self> {
        let mut file = File::open(path)?;
        let offset = self::search(&mut file)?;

        Self::parse(file, offset)
    }

    fn parse(mut file: File, offset: u64) -> ParseResult<Self> {
        file.seek(SeekFrom::Start(offset))?;

        // Read and parse the possible ID3v2 header
        let mut header_raw = [0; 10];
        file.read_exact(&mut header_raw)?;

        let mut header = TagHeader::parse(header_raw)?;

        if header.major() == 2 {
            // TODO: Upgrade ID3v2.2 tags to ID3v2.3.
            return Err(ParseError::Unsupported);
        }

        // Then get the full tag data. If the size is invalid, then we will just truncate it.
        let mut tag_data = vec![0; header.size()];
        let read = file.read(&mut tag_data)?;
        tag_data.truncate(read);

        // Begin body parsing. This is where the data becomes a stream instead of a vector.
        let mut stream = BufStream::new(&tag_data);

        let (ext_header, frames) = {
            if header.major() <= 3 && header.flags().unsync {
                // ID3v2.3 tag-specific unsynchronization, decode the stream here.
                parse_body(&mut header, BufStream::new(&syncdata::decode(&mut stream)))
            } else {
                parse_body(&mut header, stream)
            }
        };

        Ok(Tag {
            file: Some(file),
            offset,
            header,
            ext_header,
            frames,
        })
    }

    pub fn version(&self) -> (u8, u8) {
        (self.header.major(), self.header.minor())
    }

    pub fn size(&self) -> usize {
        self.header.size()
    }

    pub fn frames(&self) -> &FrameMap {
        &self.frames
    }

    pub fn frames_mut(&mut self) -> &mut FrameMap {
        &mut self.frames
    }
}

fn search(file: &mut File) -> ParseResult<u64> {
    let mut stream = BufReader::new(file);

    // The most common location for ID3v2 tags is at the beginning of a file.
    let mut id = [0; 3];
    stream.read_exact(&mut id)?;

    if id == tag::ID_HEADER {
        return Ok(0);
    }

    // In some cases, an ID3v2 tag can exist after some other data, so
    // we search for a tag until the EOF.

    // TODO: Searching process should be made more format-specific

    let mut offset = 0;

    while let Ok(()) = stream.read_exact(&mut id) {
        if id.eq(tag::ID_HEADER) {
            return Ok(offset);
        }

        offset += 3;
    }

    // There is no tag.
    Err(ParseError::NotFound)
}

fn parse_body(
    tag_header: &mut TagHeader,
    mut stream: BufStream,
) -> (Option<ExtendedHeader>, FrameMap) {
    let mut ext_header = None;

    if tag_header.flags().extended {
        // Certain taggers will flip the extended header flag without writing one,
        // so if parsing fails then we correct the flag.
        match ExtendedHeader::parse(&mut stream, tag_header.major()) {
            Ok(header) => ext_header = Some(header),
            Err(_) => tag_header.flags_mut().extended = false
        }
    }

    // Now try parsing our frames,
    let mut frames = FrameMap::new();

    while let Ok(frame) = frames::new(&tag_header, &mut stream) {
        frames.add(frame);
    }

    (ext_header, frames)
}

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug)]
pub enum ParseError {
    IoError(io::Error),
    MalformedData,
    Unsupported,
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
