pub mod apic;
pub mod bin;
pub mod comments;
pub mod events;
pub mod geob;
pub mod lyrics;
pub mod stats;
pub mod owner;
mod string;
pub mod text;
pub mod time;
pub mod url;

pub use apic::AttatchedPictureFrame;
pub use bin::{FileIdFrame, PrivateFrame, RawFrame};
pub use comments::CommentsFrame;
pub use events::EventTimingCodesFrame;
pub use geob::GeneralObjectFrame;
pub use owner::{OwnershipFrame, TermsOfUseFrame};
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

pub(crate) fn new(tag_header: &TagHeader, data: &[u8]) -> Option<Box<dyn Id3Frame>> {
    // Headers need to look ahead in some cases for sanity checking, so we give it the
    // entire slice instead of the first ten bytes.
    let frame_header = FrameHeader::new(tag_header, data)?;

    let data = &data[10..frame_header.frame_size + 10];

    // TODO: Handle duplicate frames
    // TODO: Handle iTunes weirdness
    // TODO: Add a unified property interface [Likely through a trait & enums]

    match decode_frame(tag_header, &frame_header, data) {
        // Frame data was decoded, handle frame using that
        FrameData::Some(new_data) => create_frame(frame_header, &new_data),

        // Frame data is not encoded, use normal data
        FrameData::None => create_frame(frame_header, data),

        // Unsupported, return a raw frame
        FrameData::Unsupported => Some(Box::new(RawFrame::new(frame_header, data)))
    }
}

enum FrameData {
    Some(Vec<u8>),
    None,
    Unsupported
}

fn decode_frame(tag_header: &TagHeader, frame_header: &FrameHeader, data: &[u8]) -> FrameData {
    let mut result = FrameData::None;

    // Frame-Specific Unsynchronization [If the tag does not already unsync everything]
    if frame_header.unsync && !tag_header.unsync {
        result = FrameData::Some(syncdata::decode(data));
    }

    // Encryption and Compression. Not implemented for now.
    if frame_header.compressed || frame_header.encrypted {
        return FrameData::Unsupported;
    }

    result
}

fn create_frame(mut header: FrameHeader, data: &[u8]) -> Option<Box<dyn Id3Frame>> {
    // Flags can modify where the true data of a frame can begin, so we have to check for that
    let mut start = 0;

    // Group Identifier, this *probably* comes before any other data.
    // We don't bother with it.
    if header.has_group && !data.is_empty() {
        start += 1;
    }

    // External Size Identifier. In ID3v2.4, this is a seperate flag, while in ID3v2.3,
    // its implied when compression is enabled.
    if (header.has_data_len || header.compressed) && (data.len() - start) >= 4 {
        let size = syncdata::to_size(&data[start..start + 4]);

        // Validate that this new size is OK
        if size > 0 && size < data.len() {
            header.frame_size = size;
            start += 4;
        }
    }

    // Ensure that our starting position isn't outside the data.
    // This probably shouldn't happen, but better safe than sorry.
    if start > data.len() {
        start = 0;
    }

    // Make sure that we won't overread the data with a malformed frame
    if header.frame_size == 0 || header.frame_size > data.len() {
        return None;
    }

    let data = &data[start..];

    build_frame(header, data) 
}

fn build_frame(header: FrameHeader, data: &[u8]) -> Option<Box<dyn Id3Frame>> {
    // To build our frame, we have to manually go through and determine what kind of
    // frame to create based on the frame id. There are many frame possibilities, so
    // there are many match arms.

    let frame: Box<dyn Id3Frame> = match header.frame_id.as_str() {
        // --- Text Information [Frames 4.2] ---

        // User-Defined Text Informations [Frames 4.2.6]
        "TXXX" => Box::new(UserTextFrame::new(header, data)?),

        // Involved People List & Musician Credits List [Frames 4.2.2]
        // These can all be mapped to the same frame [Including the legacy IPLS frame]
        "IPLS" | "TIPL" | "TMCL" => Box::new(CreditsFrame::new(header, data)?),

        // Apple's WFED (Podcast URL), MVNM (Movement Name), MVIN (Movement Number),
        // and GRP1 (Grouping) frames are all actually text frames
        "WFED" | "MVNM" | "MVIN" | "GRP1" => Box::new(TextFrame::new(header, data)?),

        // Generic Text Information
        id if id.starts_with('T') => Box::new(TextFrame::new(header, data)?),

        // --- URL Link [Frames 4.3] ---

        // User-Defined URL Link [Frames 4.3.2]
        "WXXX" => Box::new(UserUrlFrame::new(header, data)?),

        // Generic URL Link
        id if id.starts_with('W') => Box::new(UrlFrame::new(header, data)?),

        // --- Other Frames ---

        // Unique File Identifier [Frames 4.1]
        "UFID" => Box::new(FileIdFrame::new(header, data)?),

        // Event timing codes [Frames 4.5]
        "ETCO" => Box::new(EventTimingCodesFrame::new(header, data)?),

        // Unsynchronized Lyrics [Frames 4.8]
        "USLT" => Box::new(UnsyncLyricsFrame::new(header, data)?),

        // Unsynchronized Lyrics [Frames 4.9]
        "SYLT" => Box::new(SyncedLyricsFrame::new(header, data)?),

        // Comments [Frames 4.10]
        "COMM" => Box::new(CommentsFrame::new(header, data)?),

        // TODO: Relative Volume Adjustment [Frames 4.11]

        // Attatched Picture [Frames 4.14]
        "APIC" => Box::new(AttatchedPictureFrame::new(header, data)?),

        // General Encapsulated Object [Frames 4.15]
        "GEOB" => Box::new(GeneralObjectFrame::new(header, data)?),

        // Play Counter [Frames 4.16]
        "PCNT" => Box::new(PlayCounterFrame::new(header, data)?),

        // Popularimeter [Frames 4.17]
        "POPM" => Box::new(PopularimeterFrame::new(header, data)?),

        // TODO: [Maybe] Linked info frame [Frames 4.20]
        // Terms of use frame [Frames 4.22]

        "USER" => Box::new(TermsOfUseFrame::new(header, data)?),

        // Ownership frame [Frames 4.23]
        "OWNE" => Box::new(OwnershipFrame::new(header, data)?),

        // TODO: [Maybe] Commercial Frame [Frames 4.24]

        // Private Frame [Frames 4.27]
        "PRIV" => Box::new(PrivateFrame::new(header, data)?),
        
        // TODO: Chapter and TOC Frames

        // Unknown, return raw frame
        _ => Box::new(RawFrame::new(header, data)),
    };

    Some(frame)
}

pub struct FrameHeader {
    frame_id: String,
    frame_size: usize,
    tag_should_discard: bool,
    file_should_discard: bool,
    read_only: bool,
    has_group: bool,
    compressed: bool,
    encrypted: bool,
    unsync: bool,
    has_data_len: bool
}

impl FrameHeader {
    fn new(header: &TagHeader, data: &[u8]) -> Option<Self> {
        // Frame header formats diverge quite signifigantly across ID3v2 versions, 
        // so we need to handle them seperately

        match header.major {
            3 => new_header_v3(data),
            4 => new_header_v4(data),
            _ => None // TODO: Parse ID3v2.2 headers
        }
    }
}

fn new_header_v3(data: &[u8]) -> Option<FrameHeader> {
    let frame_id = new_frame_id(&data[0..4])?;
    let frame_size = raw::to_size(&data[4..8]);

    let stat_flags = data[8];
    let format_flags = data[9];

    Some(FrameHeader {
        frame_id,
        frame_size,
        tag_should_discard: raw::bit_at(0, stat_flags),
        file_should_discard: raw::bit_at(1, stat_flags),
        read_only: raw::bit_at(2, stat_flags),
        compressed: raw::bit_at(0, format_flags),
        encrypted: raw::bit_at(1, format_flags),
        has_group: raw::bit_at(2, format_flags),
        unsync: false,
        has_data_len: false,
    })
}

fn new_header_v4(data: &[u8]) -> Option<FrameHeader> {
    let frame_id = new_frame_id(&data[0..4])?;

    // ID3v2.4 sizes SHOULD Be syncsafe, but iTunes is a special little snowflake and wrote
    // old ID3v2.3 sizes instead for a time. Handle that.
    let mut frame_size = syncdata::to_size(&data[4..8]);

    if frame_size >= 0x80 && !is_frame_id(&data[frame_size + 10..frame_size + 14]) && data[frame_size + 10] != 0 {
        let raw_size = raw::to_size(&data[4..8]);

        if is_frame_id(&data[raw_size + 10..raw_size + 14]) {
            frame_size = raw_size;
        }
    }

    let stat_flags = data[8];
    let format_flags = data[9];

    Some(FrameHeader {
        frame_id,
        frame_size,
        tag_should_discard: raw::bit_at(1, stat_flags),
        file_should_discard: raw::bit_at(2, stat_flags),
        read_only: raw::bit_at(3, stat_flags),
        has_group: raw::bit_at(1, format_flags),
        compressed: raw::bit_at(4, format_flags),
        encrypted: raw::bit_at(5, format_flags),
        unsync: raw::bit_at(6, format_flags),
        has_data_len: raw::bit_at(7, format_flags),    
    })
}

fn new_frame_id(frame_id: &[u8]) -> Option<String> {
    if !is_frame_id(frame_id) {
        return None;
    }

    String::from_utf8(frame_id.to_vec()).ok()
}

fn is_frame_id(frame_id: &[u8]) -> bool {
    for ch in frame_id {
        if !(b'A'..b'Z').contains(ch) && !(b'0'..b'9').contains(ch) {
            return false;
        }
    }

    true
}