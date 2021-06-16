pub mod frame_map;
pub mod header;
pub mod string;
mod internal;

pub use frame_map::FrameMap;
pub use header::{FrameFlags, FrameHeader};
pub use internal::*;

pub use apic::AttatchedPictureFrame;
pub use bin::{FileIdFrame, PrivateFrame, RawFrame};
pub use chapters::{ChapterFrame, TableOfContentsFrame};
pub use comments::CommentsFrame;
pub use events::EventTimingCodesFrame;
pub use geob::GeneralObjectFrame;
pub use lyrics::{SyncedLyricsFrame, UnsyncLyricsFrame};
pub use owner::{OwnershipFrame, TermsOfUseFrame};
pub use podcast::PodcastFrame;
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
    fn parse(&mut self, header: &TagHeader, data: &[u8]) -> Result<(), ParseError>;
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
    let mut frame_header = FrameHeader::parse(tag_header.major(), data)?;

    // Make sure that we won't overread the data with a malformed frame
    if frame_header.size() + 10 > data.len() {
        return Err(ParseError::InvalidData);
    }

    let data = &data[10..frame_header.size() + 10];

    // TODO: Handle iTunes insanity

    match decode_frame(tag_header, &mut frame_header, data) {
        // Frame data was decoded, handle frame using that
        DecodedData::Some(new_data) => create_frame(tag_header, frame_header, &new_data),

        // Frame data is not encoded, use normal data
        DecodedData::None => create_frame(tag_header, frame_header, data),

        // Unsupported, return a raw frame
        DecodedData::Unsupported => Ok(Box::new(RawFrame::with_raw(frame_header, data))),
    }
}

enum DecodedData {
    Some(Vec<u8>),
    None,
    Unsupported,
}

fn decode_frame(tag_header: &TagHeader, frame_header: &mut FrameHeader, data: &[u8]) -> DecodedData {
    let mut result = DecodedData::None;

    // Frame-Specific Unsynchronization [If the tag does not already unsync everything]
    if frame_header.flags().unsync && !tag_header.flags().unsync {
        // Update the frame size to reflect the new data length
        let data = syncdata::decode(data);
        *frame_header.size_mut() = data.len();
        result = DecodedData::Some(data);
    }

    // Encryption and Compression. Not implemented for now.
    if frame_header.flags().compressed || frame_header.flags().encrypted {
        return DecodedData::Unsupported;
    }

    result
}

fn create_frame(
    tag_header: &TagHeader,
    frame_header: FrameHeader,
    data: &[u8],
) -> Result<Box<dyn Frame>, ParseError> {
    // Flags can modify where the true data of a frame can begin, so we have to check for that
    let mut start = 0;
    let frame_flags = frame_header.flags();

    // Group Identifier, this *probably* comes before any other data.
    // We don't bother with it.
    if frame_flags.has_group && !data.is_empty() {
        start += 1;
    }

    // External Size Identifier. In ID3v2.4, this is a seperate flag, while in ID3v2.3,
    // its implied when compression is enabled.
    // We also dont bother with it as we dynamically determine the "true" size in
    // decode_frame
    if (frame_flags.has_data_len || frame_flags.compressed) && (data.len() - start) >= 4 {
        start += 4;
    }

    // Ensure that our starting position isn't outside the data.
    // This probably shouldn't happen, but better safe than sorry.
    if start > data.len() {
        start = 0;
    }

    let data = &data[start..];

    build_frame(tag_header, frame_header, data)
}

fn build_frame(
    tag_header: &TagHeader,
    frame_header: FrameHeader,
    data: &[u8],
) -> Result<Box<dyn Frame>, ParseError> {
    // To build our frame, we have to manually go through and determine what kind of
    // frame to create based on the frame id. There are many frame possibilities, so
    // there are many match arms.

    let mut frame: Box<dyn Frame> = match frame_header.id().as_str() {
        // --- Text Information [Frames 4.2] ---

        // Involved People List & Musician Credits List [Frames 4.2.2]
        // These can all be mapped to the same frame [Including the legacy IPLS frame]
        "IPLS" | "TIPL" | "TMCL" => Box::new(CreditsFrame::with_header(frame_header)),

        // User-Defined Text Informations [Frames 4.2.6]
        "TXXX" => Box::new(UserTextFrame::with_header(frame_header)),

        // Generic Text Information
        id if TextFrame::is_text(id) => Box::new(TextFrame::with_header(frame_header)),

        // --- URL Link [Frames 4.3] ---

        // User-Defined URL Link [Frames 4.3.2]
        "WXXX" => Box::new(UserUrlFrame::with_header(frame_header)),

        // Generic URL Link
        id if id.starts_with('W') => Box::new(UrlFrame::with_header(frame_header)),

        // --- Other Frames ---

        // Unique File Identifier [Frames 4.1]
        "UFID" => Box::new(FileIdFrame::with_header(frame_header)),

        // Event timing codes [Frames 4.5]
        "ETCO" => Box::new(EventTimingCodesFrame::with_header(frame_header)),

        // Unsynchronized Lyrics [Frames 4.8]
        "USLT" => Box::new(UnsyncLyricsFrame::with_header(frame_header)),

        // Unsynchronized Lyrics [Frames 4.9]
        "SYLT" => Box::new(SyncedLyricsFrame::with_header(frame_header)),

        // Comments [Frames 4.10]
        "COMM" => Box::new(CommentsFrame::with_header(frame_header)),

        // TODO: Relative Volume Adjustment [Frames 4.11]

        // Attatched Picture [Frames 4.14]
        "APIC" => Box::new(AttatchedPictureFrame::with_header(frame_header)),

        // General Encapsulated Object [Frames 4.15]
        "GEOB" => Box::new(GeneralObjectFrame::with_header(frame_header)),

        // Play Counter [Frames 4.16]
        "PCNT" => Box::new(PlayCounterFrame::with_header(frame_header)),

        // Popularimeter [Frames 4.17]
        "POPM" => Box::new(PopularimeterFrame::with_header(frame_header)),

        // TODO: [Maybe] Linked info frame [Frames 4.20]

        // Terms of use frame [Frames 4.22]
        "USER" => Box::new(TermsOfUseFrame::with_header(frame_header)),

        // Ownership frame [Frames 4.23]
        "OWNE" => Box::new(OwnershipFrame::with_header(frame_header)),

        // TODO: [Maybe] Commercial Frame [Frames 4.24]

        // Private Frame [Frames 4.27]
        "PRIV" => Box::new(PrivateFrame::with_header(frame_header)),

        // iTunes Podcast Frame
        "PCST" => Box::new(PodcastFrame::with_header(frame_header)),

        // Chapter Frame [ID3v2 Chapter Frame Addendum 3.1]
        "CHAP" => Box::new(ChapterFrame::with_header(frame_header)),

        // Table of Contents Frame [ID3v2 Chapter Frame Addendum 3.2]
        "CTOC" => Box::new(TableOfContentsFrame::with_header(frame_header)),

        // Unknown, return raw frame
        _ => Box::new(RawFrame::with_header(frame_header)),
    };

    frame.parse(tag_header, data)?;

    Ok(frame)
}

#[cfg(test)]
mod tests {
    use crate::file::File;
    use std::env;

    #[test]
    fn decode_unsync_data() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/unsync.mp3";
        let mut file = File::open(&path).unwrap();
        let tag = file.id3v2().unwrap();
        let frames = tag.frames();

        assert_eq!(frames["TIT2"].to_string(), "My babe just cares for me");
        assert_eq!(frames["TPE1"].to_string(), "Nina Simone");
        assert_eq!(frames["TALB"].to_string(), "100% Jazz");
        assert_eq!(frames["TRCK"].to_string(), "03");
        assert_eq!(frames["TLEN"].to_string(), "216000");
    }

    #[test]
    fn handle_itunes_frame_sizes() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/itunes_sizes.mp3";
        let mut file = File::open(&path).unwrap();
        let tag = file.id3v2().unwrap();
        let frames = tag.frames();

        assert_eq!(frames["TIT2"].to_string(), "Sunshine Superman");
        assert_eq!(frames["TPE1"].to_string(), "Donovan");
        assert_eq!(frames["TALB"].to_string(), "Sunshine Superman");
        assert_eq!(frames["TRCK"].to_string(), "1");   
    }
}