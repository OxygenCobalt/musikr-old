mod apic;
mod string;
mod text;

use crate::id3::util;
use crate::id3::Id3TagHeader;

use apic::ApicFrame;
use text::TextFrame;

pub trait Id3Frame {
    fn code(&self) -> &String;
    fn size(&self) -> usize;
    fn format(&self) -> String;
}

pub struct Id3FrameHeader {
    code: String,
    size: usize,

    // Temporary flags until these are used
    #[allow(dead_code)]
    stat_flags: u8,

    #[allow(dead_code)]
    encode_flags: u8,
}

pub(super) fn new(_header: &Id3TagHeader, data: &[u8]) -> Option<Box<dyn Id3Frame>> {
    let frame_header = Id3FrameHeader::from(&data[0..10])?;

    // Make sure that we won't overread the data with a malformed frame
    if frame_header.size == 0 || (frame_header.size + 10) > data.len() {
        return None;
    }

    let data = &data[10..(frame_header.size + 10)];

    // Now we have to manually go through and determine what kind of frame to create based
    // on the code. There are many frame possibilities, so theres alot of if blocks.

    // TODO: Handle compressed frames

    // Text Identification [Frames 4.2]

    if frame_header.code.starts_with('T') {
        return Some(Box::new(TextFrame::from(frame_header, data)));
    }

    // Attatched Picture [Frames 4.15]

    if frame_header.code == "APIC" {
        return Some(Box::new(ApicFrame::from(frame_header, data)));
    }

    return None;
}

impl Id3FrameHeader {
    fn from(data: &[u8]) -> Option<Id3FrameHeader> {
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

        return Some(Id3FrameHeader {
            code,
            size,
            stat_flags,
            encode_flags,
        });
    }
}
