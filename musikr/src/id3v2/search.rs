use crate::file::File;
use crate::id3v2::{header, ParseError, Tag};
use std::io::{self, Error, ErrorKind};

const BLOCK_SIZE: usize = 1024;

pub fn search(file: &mut File) -> io::Result<Tag> {
    // The most common location for ID3v2 tags is at the beginning of a file.
    let mut id = [0; 3];
    file.read_into(&mut id)?;

    if id.eq(header::ID_HEADER) {
        return Tag::new(file, 0);
    }

    // In some cases, an ID3v2 tag can exist after some other data, so
    // we search for a tag until the EOF.

    // TODO: Try searching for a footer?

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
