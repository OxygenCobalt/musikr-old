mod apic;
mod string;
mod text;

use super::util;
use super::ID3TagHeader;

use apic::APICFrame;
use text::TextFrame;

pub trait ID3Frame {
    fn code(&self) -> &String;
    fn size(&self) -> usize;
    fn format(&self) -> String;
}

pub struct FrameHeader {
    code: String,
    size: usize,

    // Temporary flags until these are used
    #[allow(dead_code)]
    stat_flags: u8,

    #[allow(dead_code)]
    encode_flags: u8,
}

pub(super) fn new(header: &ID3TagHeader, data: &[u8]) -> Option<Box<dyn ID3Frame>> {
    // First create our header, which we will pass to all of the frame
    // implementations that we produce.
    let code = create_frame_code(&data[0..4])?;
    let size = util::size_decode(&data[4..8]);

    // Make sure that we won't overread the data with a malformed frame
    if size == 0 || (size + 10) > data.len() {
        return None;
    }

    let stat_flags = data[8];
    let encode_flags = data[9];

    let header = FrameHeader {
        code,
        size,
        stat_flags,
        encode_flags,
    };

    let data = &data[10..(size + 10)];

    // This is where things get messy, as we have to manually check each frame code and then create
    // the corresponding ID3 frame alongside it.
    
    if header.code.starts_with('T') {
        return Some(Box::new(TextFrame::from(header, data)));
    }

    if header.code == "APIC" {
        return Some(Box::new(APICFrame::from(header, data)));
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
        Err(_) => None,
    };
}
