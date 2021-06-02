pub mod bin;
pub mod comments;
pub mod geob;
pub mod text;
pub mod url;
pub mod apic;
pub mod lyrics;
mod string;

pub use bin::{FileIdFrame, RawFrame};
pub use comments::CommentsFrame;
pub use apic::AttatchedPictureFrame;
pub use geob::GeneralObjectFrame;
pub use lyrics::UnsyncLyricsFrame;
pub use text::{CreditsFrame, TextFrame, UserTextFrame};
pub use url::{UrlFrame, UserUrlFrame};

use std::fmt::Display;

use crate::id3::{util, TagHeader};

pub trait Id3Frame: Display {
    fn id(&self) -> &String;
    fn size(&self) -> usize;
}

pub(super) fn new(header: &TagHeader, data: &[u8]) -> Option<Box<dyn Id3Frame>> {
    let frame_header = Id3FrameHeader::from(header, &data[0..10])?;

    // Make sure that we won't overread the data with a malformed frame
    if frame_header.frame_size == 0 || (frame_header.frame_size + 10) > data.len() {
        return None;
    }

    let data = &data[10..(frame_header.frame_size + 10)];

    // Now we have to manually go through and determine what kind of frame to create based
    // on the frame id. There are many frame possibilities, so there are many if blocks.

    // TODO: Handle compressed frames
    // TODO: Handle duplicate frames
    // TODO: Handle unsynchonization
    // TODO: Handle iTunes weirdness

    let frame_id = &frame_header.frame_id;

    // Unique File Identifier [Frames 4.1]

    if frame_id == "UFID" {
        return Some(Box::new(FileIdFrame::from(frame_header, data)));
    }

    // --- Text Information [Frames 4.2] ---

    // Involved People List & Musician Credits List [Frames 4.2.2]
    // Both of these lists can correspond to the same frame.

    if frame_id == "TIPL" || frame_id == "IPLS" || frame_id == "TMCL" {
        return Some(Box::new(CreditsFrame::from(frame_header, data)));
    }

    // All text frames begin with 'T', but apple's proprietary WFED (Podcast URL), MVNM (Movement Name),
    // MVIN (Movement Number), and GRP1 (Grouping) frames are all text frames as well.
    if frame_id.starts_with('T')
        || frame_id == "WFED"
        || frame_id == "MVNM"
        || frame_id == "MVIN"
        || frame_id == "GRP1"
    {
        // User-Defined Text Info [Frames 4.2.6]

        if frame_id == "TXXX" {
            return Some(Box::new(UserTextFrame::from(frame_header, data)));
        }

        return Some(Box::new(TextFrame::from(frame_header, data)));
    }

    // --- URL Link [Frames 4.3] ---

    if frame_id.starts_with('W') {
        // User-Defined URL [Frames 4.3.2]

        if frame_id == "WXXX" {
            return Some(Box::new(UserUrlFrame::from(frame_header, data)));
        }

        return Some(Box::new(UrlFrame::from(frame_header, data)));
    }

    // Unsynchronized Lyrics [Frames 4.8]

    if frame_id == "USLT" {
        return Some(Box::new(UnsyncLyricsFrame::from(frame_header, data)));
    }

    // Comments [Frames 4.10]

    if frame_id == "COMM" {
        return Some(Box::new(CommentsFrame::from(frame_header, data)));
    }

    // Attatched Picture [Frames 4.14]

    if frame_id == "APIC" {
        return Some(Box::new(AttatchedPictureFrame::from(frame_header, data)));
    }

    // General Encapsulated Object [Frames 4.15]

    if frame_id == "GEOB" {
        return Some(Box::new(GeneralObjectFrame::from(frame_header, data)));
    }

    // Not supported, return a raw frame
    return Some(Box::new(RawFrame::from(frame_header, data)));
}

pub struct Id3FrameHeader {
    frame_id: String,
    frame_size: usize,
    stat_flags: u8,
    format_flags: u8,
}

impl Id3FrameHeader {
    fn from(header: &TagHeader, data: &[u8]) -> Option<Id3FrameHeader> {
        let frame_id = &data[0..4];

        // Make sure that our frame code is 4 valid uppercase ASCII chars
        for &ch in frame_id {
            if (ch < b'A' || ch > b'Z') && (ch < b'0' || ch > b'9') {
                return None;
            }
        }

        // UTF-8 is the closest to ASCII that rust supports
        let frame_id = String::from_utf8(frame_id.to_vec()).ok()?;

        // ID3v2.4 uses syncsafe on frame sizes while other versions don't
        let frame_size = if header.major == 4 {
            util::syncsafe_decode(&data[4..8])
        } else {
            util::size_decode(&data[4..8])
        };

        let stat_flags = data[8];
        let format_flags = data[9];

        return Some(Id3FrameHeader {
            frame_id,
            frame_size,
            stat_flags,
            format_flags,
        });
    }
}
