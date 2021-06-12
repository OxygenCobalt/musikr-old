use crate::file::File;
use crate::id3v2::{header, Tag, ParseError};
use std::io::{self, Error, ErrorKind};
use std::convert::TryInto;

const BLOCK_SIZE: usize = 1024;

pub fn search(file: &mut File) -> io::Result<Tag> {
    // The most common location for ID3v2 tags is at the beginning of a file.
    let mut id = [0; 3];
    file.read_into(&mut id)?;

    if id.eq(header::IDENTIFIER) {
        return Tag::new(file, 0);
    }

    // That didn't work. The tag may be after some data

    if let Some(offset) = search_fowards(file) {
        return Tag::new(file, offset);
    }

    // TODO: Try to search backwards for a footer.
    // TODO: Handle SEEK frames

    // There is no tag.
    Err(Error::new(ErrorKind::NotFound, ParseError::NotFound))
}

fn search_fowards(file: &mut File) -> Option<u64> {
    // In some cases, an ID3v2 tag can exist after some other data, so 
    // we search for the tag until the first MPEG frame.

    let mut id = [0; 3];
    let mut sync_pair = [0; 2];
    let mut pos = 0;

    // Read 1024-sized blocks until we reach the EOF, incomplete blocks will be ignored.
    while let Ok(block) = file.read_bytes(BLOCK_SIZE) {
        for (i, byte) in block.iter().enumerate() {
            id[0] = id[1];
            id[1] = id[2];
            id[2] = *byte;

            if id.eq(header::IDENTIFIER) {
                // Found a tag. Now we need to simply get the offset casted as a u64.
                // This cast should probably always work since usize is at most a u64.
                let pos: u64 = pos.try_into().unwrap();
                let i: u64 = i.try_into().unwrap();

                return Some(pos + i - 2);
            }

            sync_pair[0] = sync_pair[1];
            sync_pair[1] = *byte;

            if is_mpeg_sync(sync_pair[0], sync_pair[1]) {
                // We're likely in MPEG data, game over
                return None;
            }
        }

        pos += BLOCK_SIZE;
    }

    None
}

fn is_mpeg_sync(a: u8, b: u8) -> bool {
    a == 0xFF && b != 0xFF && (b & 0xE0) == 0xE0
}