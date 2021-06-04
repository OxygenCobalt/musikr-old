pub mod apic;
pub mod bin;
pub mod comments;
pub mod events;
pub mod geob;
pub mod lyrics;
pub mod stats;
mod string;
pub mod text;
pub mod time;
pub mod url;

pub use apic::AttatchedPictureFrame;
pub use bin::{FileIdFrame, PrivateFrame, RawFrame};
pub use comments::CommentsFrame;
pub use events::EventTimingCodesFrame;
pub use geob::GeneralObjectFrame;
pub use lyrics::{SyncedLyricsFrame, UnsyncLyricsFrame};
pub use stats::{PlayCounterFrame, PopularimeterFrame};
pub use text::{CreditsFrame, TextFrame, UserTextFrame};
pub use url::{UrlFrame, UserUrlFrame};

use crate::id3::{syncdata, TagHeader};
use crate::raw;
use std::fmt::Display;

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

    // TODO: Handle compressed frames
    // TODO: Handle duplicate frames
    // TODO: Handle unsynchonization
    // TODO: Handle iTunes weirdness
    // TODO: Make frame creation return defaults when there isn't enough data
    // TODO: Add readable frame names

    // Now we have to manually go through and determine what kind of frame to create based
    // on the frame id. There are many frame possibilities, so there are many match arms.

    let frame: Box<dyn Id3Frame> = match frame_header.frame_id.as_str() {
        // --- Text Information [Frames 4.2] ---

        // User-Defined Text Informations [Frames 4.2.6]
        "TXXX" => Box::new(UserTextFrame::new(frame_header, data)),

        // Involved People List & Musician Credits List [Frames 4.2.2]
        // These can all be mapped to the same frame [Including the legacy IPLS frame]
        "IPLS" | "TIPL" | "TMCL" => Box::new(CreditsFrame::new(frame_header, data)),

        // Apple's WFED (Podcast URL), MVNM (Movement Name), MVIN (Movement Number),
        // and GRP1 (Grouping) frames are all actually text frames
        "WFED" | "MVNM" | "MVIN" | "GRP1" => Box::new(TextFrame::new(frame_header, data)),
        
        // Generic Text Information
        id if id.starts_with('T') => Box::new(TextFrame::new(frame_header, data)),

        // --- URL Link [Frames 4.3] ---

        // User-Defined URL Link [Frames 4.3.2]
        "WXXX" => Box::new(UserUrlFrame::new(frame_header, data)),

        // Generic URL Link
        id if id.starts_with('W') => Box::new(UrlFrame::new(frame_header, data)),

        // --- Other Frames ---

        // Unique File Identifier [Frames 4.1]
        "UFID" => Box::new(FileIdFrame::new(frame_header, data)),

        // Event timing codes [Frames 4.5]
        "ETCO" => Box::new(EventTimingCodesFrame::new(frame_header, data)),

        // Unsynchronized Lyrics [Frames 4.8]
        "USLT" => Box::new(UnsyncLyricsFrame::new(frame_header, data)),

        // Unsynchronized Lyrics [Frames 4.9]
        "SYLT" => Box::new(SyncedLyricsFrame::new(frame_header, data)),

        // Comments [Frames 4.10]
        "COMM" => Box::new(CommentsFrame::new(frame_header, data)),

        // TODO: Relative Volume Adjustment [Frames 4.11]

        // Attatched Picture [Frames 4.14]
        "APIC" => Box::new(AttatchedPictureFrame::new(frame_header, data)),

        // General Encapsulated Object [Frames 4.15]
        "GEOB" => Box::new(GeneralObjectFrame::new(frame_header, data)),

        // Play Counter [Frames 4.16]
        "PCNT" => Box::new(PlayCounterFrame::new(frame_header, data)),

        // Popularimeter [Frames 4.17]
        "POPM" => Box::new(PopularimeterFrame::new(frame_header, data)),

        // TODO: [Maybe] Linked info frame [Frames 4.20]
        // TODO: Terms of use frame [Frames 4.22]
        // TODO: Ownership frame [Frames 4.23]
        // TODO: [Maybe] Commercial Frame [Frames 4.24]

        // Private Frame [Frames 4.27]
        "PRIV" => Box::new(PrivateFrame::new(frame_header, data)),

        // Unknown, return raw frame
        _ => Box::new(RawFrame::from(frame_header, data)),
    };

    Some(frame)
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
        for ch in frame_id {
            if !(b'A'..b'Z').contains(ch) && !(b'0'..b'9').contains(ch) {
                return None;
            }
        }

        // UTF-8 is the closest to ASCII that rust supports
        let frame_id = String::from_utf8(frame_id.to_vec()).ok()?;

        // ID3v2.4 uses syncsafe on frame sizes while other versions don't
        let frame_size = if header.major == 4 {
            syncdata::to_size(&data[4..8])
        } else {
            raw::to_size(&data[4..8])
        };

        let stat_flags = data[8];
        let format_flags = data[9];

        Some(Id3FrameHeader {
            frame_id,
            frame_size,
            stat_flags,
            format_flags,
        })
    }
}
