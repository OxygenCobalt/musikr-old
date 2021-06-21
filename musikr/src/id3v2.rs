pub mod frame_map;
pub mod frames;
mod syncdata;
pub mod tag;

use crate::file::File;
use frame_map::FrameMap;
use tag::ExtendedHeader;
use tag::TagHeader;

use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, ErrorKind};

// TODO: The current roadmap for this module:
// - Improve current frame implementation
// - Try to complete most if not all of the frame specs
// - Work on tag compat and upgrading
// - Add proper tag writing

pub struct Tag {
    header: TagHeader,
    ext_header: Option<ExtendedHeader>,
    frames: FrameMap,
}

impl Tag {
    pub fn new(file: &mut File, offset: u64) -> io::Result<Self> {
        file.seek(offset)?;

        // Read and parse the possible ID3 header
        let mut header_raw = [0; 10];
        file.read_into(&mut header_raw)?;

        let mut header = match TagHeader::parse(header_raw) {
            Ok(header) => header,
            Err(err) => return Err(io::Error::new(ErrorKind::InvalidData, err)),
        };

        // Ensure that this file is large enough to even contain this tag.
        if header.size() as u64 > file.metadata().len() - offset {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                ParseError::NotEnoughData,
            ));
        }

        let mut tag_data = file.read_vec(header.size())?;

        // Unsync is gloabl in ID3v2.3
        if header.flags().unsync && header.major() <= 3 {
            tag_data = syncdata::decode(&tag_data);
        }

        // If we have an extended header, try to parse it.
        // It can remain reasonably absent if the flag isnt set or if the parsing fails.
        let ext_header = {
            if header.flags().extended {
                match ExtendedHeader::parse(&tag_data, header.major()) {
                    Ok(header) => Some(header),

                    Err(_) => {
                        header.flags_mut().extended = false;
                        None
                    }
                }
            } else {
                None
            }
        };

        let frames = parse_frames(&header, &ext_header, &tag_data);

        Ok(Tag {
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

    pub fn unsync(&self) -> bool {
        self.header.flags().unsync
    }

    pub fn footer(&self) -> bool {
        self.header.flags().footer
    }

    pub fn ext_header(&self) -> &Option<ExtendedHeader> {
        &self.ext_header
    }
}

pub fn search(file: &mut File) -> io::Result<Tag> {
    const BLOCK_SIZE: usize = 1024;

    // The most common location for ID3v2 tags is at the beginning of a file.
    let mut id = [0; 3];
    file.read_into(&mut id)?;

    if id.eq(tag::ID_HEADER) {
        return Tag::new(file, 0);
    }

    // In some cases, an ID3v2 tag can exist after some other data, so
    // we search for a tag until the EOF.

    // TODO: Try searching for a footer?
    // TODO: Not sure how common ID3v2 is in non-mpeg files, so its possible we can do
    // specialized methods for this longer and more cumbersome searching process.

    let mut id = [0; 3];
    let mut pos = 0;

    // Read blocks up to 1024 bytes until the EOF
    while let Ok(block) = file.read_up_to(BLOCK_SIZE) {
        if block.is_empty() {
            break; // Out of data
        }

        for (i, byte) in block.iter().enumerate() {
            id[0] = id[1];
            id[1] = id[2];
            id[2] = *byte;

            if id.eq(tag::ID_HEADER) {
                // Found a possible tag. this may be a false positive though,
                // so we will only return it if the creation succeeds.
                let offset = pos as u64 + i as u64 - 2;

                if let Ok(tag) = Tag::new(file, offset) {
                    return Ok(tag);
                }
            }
        }

        pos += BLOCK_SIZE;
    }

    // There is no tag.
    Err(io::Error::new(ErrorKind::NotFound, ParseError::NotFound))
}

#[derive(Clone, Copy, Debug)]
pub enum ParseError {
    NotEnoughData,
    InvalidData,
    InvalidEncoding,
    Unsupported,
    NotFound,
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

pub struct Token {
    #[allow(dead_code)]
    inner: (),
}

impl Token {
    fn _new() -> Self {
        Token { inner: () }
    }
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
