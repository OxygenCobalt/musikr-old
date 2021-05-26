mod apic;
mod string;
mod text;

use crate::id3::util;
use crate::id3::ID3TagHeader;

use apic::APICFrame;
use text::TextFrame;

pub trait ID3Frame {
    fn code(&self) -> &String;
    fn size(&self) -> usize;
    fn format(&self) -> String;
}

pub struct ID3FrameHeader {
    code: String,
    size: usize,

    // Temporary flags until these are used
    #[allow(dead_code)]
    stat_flags: u8,

    #[allow(dead_code)]
    encode_flags: u8,
}

pub(super) fn new(header: &ID3TagHeader, data: &[u8]) -> Option<Box<dyn ID3Frame>> {
    let frame_header = ID3FrameHeader::from(&data[0..10])?;

    // Make sure that we won't overread the data with a malformed frame
    if frame_header.size == 0 || (frame_header.size + 10) > data.len() {
        return None;
    }

    let data = &data[10..(frame_header.size + 10)];

    // Now we have to manually go through and determine what kind of frame to create based
    // on the code. There are many frame possibilities, so theres alot of if blocks.

    // Text Identification

    if frame_header.code.starts_with('T') {
        return Some(Box::new(TextFrame::from(frame_header, data)));
    }

    // Attatched Picture

    if frame_header.code == "APIC" {
        return Some(Box::new(APICFrame::from(frame_header, data)));
    }

    return None;
}

impl ID3FrameHeader {
    fn from(data: &[u8]) -> Option<ID3FrameHeader> {
        let code = &data[0..4];

        // Make sure that our frame code is 4 valid uppercase ASCII chars
        for &ch in code {
            if (ch < b'A' || ch > b'Z') && (ch < b'0' || ch > b'9') {
                return None;
            }
        }

        // UTF-8 is the closest to ASCII that rust supports
        let code = String::from_utf8(code.to_vec()).ok()?;

        let size = util::size_decode(&data[4..8]);

        let stat_flags = data[8];
        let encode_flags = data[9];
    
        return Some(ID3FrameHeader {
            code,
            size,
            stat_flags,
            encode_flags
        })
    }
}
