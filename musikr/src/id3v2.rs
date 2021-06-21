pub mod frame_map;
pub mod frames;
pub mod header;
mod syncdata;

pub use frame_map::FrameMap;
pub use header::{ExtendedHeader, TagFlags, TagHeader};

use crate::err::ParseError;
use crate::file::File;
use std::io::{self, Error, ErrorKind};

// TODO: ID3v2.2 Conversions

pub struct Tag {
    header: TagHeader,
    ext_header: Option<ExtendedHeader>,
    frames: FrameMap,
}

impl Tag {
    pub fn new(file: &mut File, offset: u64) -> io::Result<Self> {
        file.seek(offset)?;

        let mut header = read_header(file)?;

        // Read out the entire tag data based on the header size, decoding it if needed.
        // Technically in ID3v2.4 unsync is only applied to frame data, but since the headers
        // are syncsafe its easier to just decode it here like we would in ID3v2.3 at the cost
        // of some efficency.
        let mut data = file.read_vec(header.size())?;

        if header.flags().unsync {
            data = syncdata::decode(&data);
        }

        // Try to parse our extended header, it can remain reasonably absent if the parsing fails.
        let ext_header = handle_ext_header(&mut header, &data);

        let frames = parse_frames(&header, &ext_header, &data);

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

pub struct Token {
    #[allow(dead_code)]
    inner: ()
}

impl Token {
    fn new() -> Self {
        Token { inner: () }
    }
}

pub fn search(file: &mut File) -> io::Result<Tag> {
    const BLOCK_SIZE: usize = 1024;

    // The most common location for ID3v2 tags is at the beginning of a file.
    let mut id = [0; 3];
    file.read_into(&mut id)?;

    if id.eq(header::ID_HEADER) {
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

            if id.eq(header::ID_HEADER) {
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
    Err(Error::new(ErrorKind::NotFound, ParseError::NotFound))
}

fn read_header(file: &mut File) -> io::Result<TagHeader> {
    // Read and parse the possible ID3 header
    let mut header_raw = [0; 10];
    file.read_into(&mut header_raw)?;

    let header = match TagHeader::parse(&header_raw) {
        Ok(header) => header,
        Err(err) => return Err(Error::new(ErrorKind::InvalidData, err)),
    };

    // Ensure that this file is large enough to even contain this tag.
    if header.size() as u64 > file.metadata().len() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            ParseError::NotEnoughData,
        ));
    }

    Ok(header)
}

fn handle_ext_header(header: &mut TagHeader, data: &[u8]) -> Option<ExtendedHeader> {
    if header.flags().extended {
        match ExtendedHeader::parse(header.major(), data) {
            Ok(header) => return Some(header),

            // Flag was incorrectly set, correct it and move on.
            Err(_) => header.flags_mut().extended = false,
        }
    }

    None
}

fn parse_frames(header: &TagHeader, ext_header: &Option<ExtendedHeader>, data: &[u8]) -> FrameMap {
    let mut frames = FrameMap::new();
    let mut frame_pos = 0;
    let mut frame_size = data.len();

    if header.flags().footer {
        frame_size -= 10;
    }

    if let Some(ext_header) = &ext_header {
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
