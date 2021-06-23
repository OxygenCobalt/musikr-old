pub mod frame_map;
pub mod frames;
mod syncdata;
pub mod tag;

use frame_map::FrameMap;
use tag::ExtendedHeader;
use tag::TagHeader;

use std::error;
use std::fs::File;
use std::path::Path;
use std::fmt::{self, Display, Formatter};
use std::io::{self, BufReader, Seek, SeekFrom, Read};

// TODO: The current roadmap for this module:
// - Try to use streams instead of slices everywhere
// - Improve current frame implementation
// - Try to complete most if not all of the frame specs
// - Work on tag compat and upgrading
// - Add proper tag writing

pub struct Tag {
    file: Option<File>,
    offset: u64,
    header: TagHeader,
    ext_header: Option<ExtendedHeader>,
    frames: FrameMap,
}

impl Tag {
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

        // Then get the full tag data. If the size is invalid, then we will just truncate it.
        let mut tag_data = vec![0; header.size()];
        let read = file.read(&mut tag_data)?;

        if read < header.size() {
            tag_data.truncate(read);
        }

        let ext_header = parse_ext_header(&mut header, &tag_data);

        // ID3v2.3 unsynchronization. Its always globally applied.
        if header.flags().unsync && header.major() <= 3 {
            tag_data = syncdata::decode(&tag_data);
        }

        let frames = parse_frames(&header, &ext_header, &tag_data);

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
            return Ok(offset)
        }

        offset += 3;
    }

    // There is no tag.
    Err(ParseError::NotFound)
}

fn parse_ext_header(header: &mut TagHeader, tag_data: &[u8]) -> Option<ExtendedHeader> {
    // If we have an extended header, try to parse it.
    // It can remain reasonably absent if the flag isnt set or if the parsing fails.
    if header.flags().extended {
        match ExtendedHeader::parse(&tag_data, header.major()) {
            Ok(header) => return Some(header),

            // Correct the extended header flag if parsing failed
            Err(_) => header.flags_mut().extended = false
        }
    }

    None
}

fn parse_frames(header: &TagHeader, ext_header: &Option<ExtendedHeader>, data: &[u8]) -> FrameMap {
    let mut frames = FrameMap::new();
    let mut frame_pos = 0;
    let mut frame_size = data.len();

    // Modify where our frame data will start if theres an extended header/footer.

    if header.flags().footer {
        frame_size -= 10;
    }

    if let Some(ext_header) = ext_header {
        frame_pos += ext_header.size();
    }

    while frame_pos < frame_size {
        // Its assumed the moment we've hit a zero, we've reached the padding
        if data[frame_pos] == 0 {
            break;
        }

        let frame = match frames::new(&header, &data[frame_pos..]) {
            Ok(frame) => frame,
            Err(_) => break,
        };

        // Add our new frame. Duplicate protection should be enforced with
        // the Id3Frame::key method and FrameMap::insert
        frame_pos += frame.size() + 10;
        frames.add(frame);
    }

    frames
}

#[derive(Debug)]
pub enum ParseError {
    IoError(io::Error),
    NotEnoughData,
    MalformedData,
    Unsupported,
    NotFound
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

pub type ParseResult<T> = Result<T, ParseError>;
