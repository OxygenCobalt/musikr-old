pub mod frame_map;
pub mod header;
mod internal;
mod string;

pub use frame_map::FrameMap;
pub use header::{FrameFlags, FrameHeader};
pub use internal::*;

pub use apic::AttatchedPictureFrame;
pub use bin::{FileIdFrame, PrivateFrame, RawFrame};
pub use comments::CommentsFrame;
pub use events::EventTimingCodesFrame;
pub use geob::GeneralObjectFrame;
pub use lyrics::{SyncedLyricsFrame, UnsyncLyricsFrame};
pub use owner::{OwnershipFrame, TermsOfUseFrame};
pub use stats::{PlayCounterFrame, PopularimeterFrame};
pub use text::{CreditsFrame, TextFrame, UserTextFrame};
pub use url::{UrlFrame, UserUrlFrame};

use crate::id3v2::{syncdata, ParseError, TagHeader};
use std::any::Any;
use std::fmt::Display;

// The id3v2::Frame downcasting system is derived from downcast-rs.
// https://github.com/marcianx/downcast-rs

pub trait AsAny: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait Frame: Display + AsAny {
    fn id(&self) -> &String;
    fn size(&self) -> usize;
    fn flags(&self) -> &FrameFlags;
    fn key(&self) -> String;
    fn parse(&mut self, data: &[u8]) -> Result<(), ParseError>;
}

impl dyn Frame {
    pub fn is<T: Frame>(&self) -> bool {
        self.as_any().is::<T>()
    }

    pub fn cast<T: Frame>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    pub fn cast_mut<T: Frame>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

impl<T: Frame> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub(crate) fn new(tag_header: &TagHeader, data: &[u8]) -> Result<Box<dyn Frame>, ParseError> {
    // Headers need to look ahead in some cases for sanity checking, so we give it the
    // entire slice instead of the first ten bytes.
    let frame_header = FrameHeader::parse(tag_header.major, data)?;
    let data = &data[10..frame_header.size() + 10];

    // TODO: Handle iTunes insanity

    match decode_frame(tag_header, &frame_header, data) {
        // Frame data was decoded, handle frame using that
        DecodedData::Some(new_data) => create_frame(frame_header, &new_data),

        // Frame data is not encoded, use normal data
        DecodedData::None => create_frame(frame_header, data),

        // Unsupported, return a raw frame
        DecodedData::Unsupported => Ok(Box::new(RawFrame::with_raw(frame_header, data))),
    }
}

enum DecodedData {
    Some(Vec<u8>),
    None,
    Unsupported,
}

fn decode_frame(tag_header: &TagHeader, frame_header: &FrameHeader, data: &[u8]) -> DecodedData {
    let mut result = DecodedData::None;

    let frame_flags = frame_header.flags();

    // Frame-Specific Unsynchronization [If the tag does not already unsync everything]
    if frame_flags.unsync && !tag_header.flags.unsync {
        result = DecodedData::Some(syncdata::decode(data));
    }

    // Encryption and Compression. Not implemented for now.
    if frame_flags.compressed || frame_flags.encrypted {
        return DecodedData::Unsupported;
    }

    result
}

fn create_frame(mut header: FrameHeader, data: &[u8]) -> Result<Box<dyn Frame>, ParseError> {
    // Flags can modify where the true data of a frame can begin, so we have to check for that
    let mut start = 0;

    let frame_flags = header.flags();

    // Group Identifier, this *probably* comes before any other data.
    // We don't bother with it.
    if frame_flags.has_group && !data.is_empty() {
        start += 1;
    }

    // External Size Identifier. In ID3v2.4, this is a seperate flag, while in ID3v2.3,
    // its implied when compression is enabled.
    if (frame_flags.has_data_len || frame_flags.compressed) && (data.len() - start) >= 4 {
        let size = syncdata::to_size(&data[start..start + 4]);

        // Validate that this new size is OK
        if size > 0 && size < data.len() {
            header.set_size(size);
            start += 4;
        }
    }

    // Ensure that our starting position isn't outside the data.
    // This probably shouldn't happen, but better safe than sorry.
    if start > data.len() {
        start = 0;
    }

    // Make sure that we won't overread the data with a malformed frame
    if header.size() == 0 || header.size() > data.len() {
        return Err(ParseError::InvalidData);
    }

    let data = &data[start..];

    build_frame(header, data)
}

fn build_frame(header: FrameHeader, data: &[u8]) -> Result<Box<dyn Frame>, ParseError> {
    // To build our frame, we have to manually go through and determine what kind of
    // frame to create based on the frame id. There are many frame possibilities, so
    // there are many match arms.

    let mut frame: Box<dyn Frame> = match header.id().as_str() {
        // --- Text Information [Frames 4.2] ---

        // Involved People List & Musician Credits List [Frames 4.2.2]
        // These can all be mapped to the same frame [Including the legacy IPLS frame]
        "IPLS" | "TIPL" | "TMCL" => {
            Box::new(CreditsFrame::with_header(header))
        }

        // User-Defined Text Informations [Frames 4.2.6]
        "TXXX" => Box::new(UserTextFrame::with_header(header)),

        // Generic Text Information
        id if TextFrame::is_text(id) => Box::new(TextFrame::with_header(header)),

        // --- URL Link [Frames 4.3] ---

        // User-Defined URL Link [Frames 4.3.2]
        "WXXX" => Box::new(UserUrlFrame::with_header(header)),

        // Generic URL Link
        id if id.starts_with('W') => Box::new(UrlFrame::with_header(header)),

        // --- Other Frames ---

        // Unique File Identifier [Frames 4.1]
        "UFID" => Box::new(FileIdFrame::with_header(header)),

        // Event timing codes [Frames 4.5]
        "ETCO" => Box::new(EventTimingCodesFrame::with_header(header)),

        // Unsynchronized Lyrics [Frames 4.8]
        "USLT" => Box::new(UnsyncLyricsFrame::with_header(header)),

        // Unsynchronized Lyrics [Frames 4.9]
        "SYLT" => Box::new(SyncedLyricsFrame::with_header(header)),

        // Comments [Frames 4.10]
        "COMM" => Box::new(CommentsFrame::with_header(header)),

        // TODO: Relative Volume Adjustment [Frames 4.11]

        // Attatched Picture [Frames 4.14]
        "APIC" => Box::new(AttatchedPictureFrame::with_header(header)),

        // General Encapsulated Object [Frames 4.15]
        "GEOB" => Box::new(GeneralObjectFrame::with_header(header)),

        // Play Counter [Frames 4.16]
        "PCNT" => Box::new(PlayCounterFrame::with_header(header)),

        // Popularimeter [Frames 4.17]
        "POPM" => Box::new(PopularimeterFrame::with_header(header)),

        // TODO: [Maybe] Linked info frame [Frames 4.20]

        // Terms of use frame [Frames 4.22]
        "USER" => Box::new(TermsOfUseFrame::with_header(header)),

        // Ownership frame [Frames 4.23]
        "OWNE" => Box::new(OwnershipFrame::with_header(header)),

        // TODO: [Maybe] Commercial Frame [Frames 4.24]

        // Private Frame [Frames 4.27]
        "PRIV" => Box::new(PrivateFrame::with_header(header)),

        // TODO: Chapter and TOC Frames

        // Unknown, return raw frame
        _ => Box::new(RawFrame::with_header(header)),
    };

    frame.parse(data)?;

    Ok(frame)
}
