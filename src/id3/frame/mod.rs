mod text;

use super::ID3Tag;
use super::util;

use text::TextFrame;

pub trait ID3Frame {
    fn code(&self) -> &String;
    fn size(&self) -> usize;
    fn format(&self) -> String;
}

pub struct FrameHeader {
    code: String,
    size: usize,
    stat_flags: u8,
    encode_flags: u8
}

pub(super) fn new(tag: &ID3Tag, at: usize) -> Option<Box<dyn ID3Frame>> {
    // First create our header, which we will pass to all of the frame
    // implementations that we produce.

    let header_raw = &tag.data[at..(at + 10)];

    let code = create_frame_code(&header_raw[0..4])?;
    let size = util::size_decode(&header_raw[4..8]);

    // Make sure that we won't overread the vec with a malformed frame
    if size == 0 || (size + at + 10) > tag.size {
        return None;
    }

    let stat_flags = header_raw[8];
    let encode_flags = header_raw[9];

    let header = FrameHeader {
        code, size, stat_flags, encode_flags
    };

    let data = &tag.data[(at + 10)..(at + 10 + size)];

    if header.code.starts_with('T') {
        return Some(Box::new(TextFrame::from(header, data)));
    }

    return None;
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