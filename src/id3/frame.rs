use super::ID3Tag;
use super::util;

pub struct ID3Frame<'a> {
    pub code: String,
    pub size: usize,
    pub stat_flags: u8,
    pub encode_flags: u8,
    pub data: &'a [u8]
}

pub(super) fn new(tag: &ID3Tag, at: usize) -> Option<ID3Frame> {
    let header = &tag.data[at..(at + 10)];

    let code = create_frame_code(&header[0..4])?;
    let size = util::size_decode(&header[4..8]);

    // Make sure that we won't overread the vec with a malformed frame
    if size == 0 || (size + at + 10) > tag.size {
        return None;
    }

    let stat_flags = header[8];
    let encode_flags = header[9];

    let data = &tag.data[(at + 10)..(at + 10 + size)];

    return Some(ID3Frame {
        code, size, stat_flags, encode_flags, data
    });
}

fn create_frame_code(data: &[u8]) -> Option<String> {
    // Sanity check: Make sure that our frame code is 4 valid uppercase ASCII chars 
    if data.len() < 4 {
        return None;
    }

    for &ch in data {
        if (ch < b'A' || ch > b'Z') && (ch < b'0' || ch > b'9') {
            return None;
        }
    }

    // UTF-8 is the closest supported format to ASCII, so just use that
    return match String::from_utf8(data.to_vec()) {
        Ok(code) => Some(code),
        Err(_) => None
    };
}