pub mod bin;
pub mod chapters;
pub mod comments;
pub mod encoding;
pub mod events;
pub mod file;
pub mod lyrics;
pub mod owner;
pub mod podcast;
pub mod stats;
pub mod text;
pub mod time;
pub mod lang;
pub mod url;

pub use bin::{FileIdFrame, PrivateFrame, UnknownFrame};
pub use chapters::{ChapterFrame, TableOfContentsFrame};
pub use comments::CommentsFrame;
pub use events::EventTimingCodesFrame;
pub use file::{AttachedPictureFrame, GeneralObjectFrame};
pub use lyrics::{SyncedLyricsFrame, UnsyncLyricsFrame};
pub use owner::{OwnershipFrame, TermsOfUseFrame};
pub use podcast::PodcastFrame;
pub use stats::{PlayCounterFrame, PopularimeterFrame};
pub use text::{CreditsFrame, TextFrame, UserTextFrame};
pub use url::{UrlFrame, UserUrlFrame};

use crate::id3v2::{syncdata, ParseError, ParseResult, TagHeader, Token};
use crate::raw;

use std::any::Any;
use std::fmt::Display;
use std::str;

// The id3v2::Frame downcasting system is derived from downcast-rs.
// https://github.com/marcianx/downcast-rs

pub trait Frame: Display + AsAny {
    fn id(&self) -> &str {
        self.header().id_str()
    }

    fn size(&self) -> usize {
        self.header().size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header().flags()
    }

    fn key(&self) -> String;

    fn header(&self) -> &FrameHeader;
    fn header_mut(&mut self, _: Token) -> &mut FrameHeader;

    fn is_empty(&self) -> bool;
    fn render(&self, tag_header: &TagHeader) -> Vec<u8>;
}

impl dyn Frame {
    pub fn is<T: Frame>(&self) -> bool {
        self.as_any().is::<T>()
    }

    pub fn downcast<T: Frame>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    pub fn downcast_mut<T: Frame>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

pub trait AsAny: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Frame> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct FrameHeader {
    frame_id: [u8; 4],
    frame_size: usize,
    flags: FrameFlags,
}

impl FrameHeader {
    pub fn new(frame_id: &[u8; 4]) -> Self {
        Self::with_flags(frame_id, FrameFlags::default())
    }

    pub fn with_flags(frame_id: &[u8; 4], flags: FrameFlags) -> Self {
        if !is_frame_id(frame_id) {
            // It's generally better to panic here as passing a malformed ID is usually programmer error.
            panic!("A Frame ID must be exactly four valid uppercase ASCII characters or numbers.")
        }

        FrameHeader {
            frame_id: *frame_id,
            frame_size: 0,
            flags,
        }
    }

    pub(crate) fn parse_v3(data: &[u8]) -> ParseResult<Self> {
        if data.len() < 10 {
            return Err(ParseError::NotEnoughData)
        }

        let frame_id = raw::to_array(&data[0..4]);
        let frame_size = raw::to_size(&data[4..8]);
    
        let stat_flags = data[8];
        let format_flags = data[9];
    
        Ok(FrameHeader {
            frame_id,
            frame_size,
            flags: FrameFlags {
                tag_alter_preservation: raw::bit_at(7, stat_flags),
                file_alter_preservation: raw::bit_at(6, stat_flags),
                read_only: raw::bit_at(5, stat_flags),
                compressed: raw::bit_at(7, format_flags),
                encrypted: raw::bit_at(6, format_flags),
                grouped: raw::bit_at(5, format_flags),
                unsync: false,
                data_len_indicator: false,
            },
        })
    }

    pub(crate) fn parse_v4(data: &[u8]) -> ParseResult<Self> {
        if data.len() < 10 {
            return Err(ParseError::NotEnoughData)
        }

        let frame_id = raw::to_array(&data[0..4]);
        let frame_size = syncdata::to_size(&data[4..8]);

        let stat_flags = data[8];
        let format_flags = data[9];
    
        Ok(FrameHeader {
            frame_id,
            frame_size,
            flags: FrameFlags {
                tag_alter_preservation: raw::bit_at(6, stat_flags),
                file_alter_preservation: raw::bit_at(5, stat_flags),
                read_only: raw::bit_at(4, stat_flags),
                grouped: raw::bit_at(6, format_flags),
                compressed: raw::bit_at(3, format_flags),
                encrypted: raw::bit_at(2, format_flags),
                unsync: raw::bit_at(1, format_flags),
                data_len_indicator: raw::bit_at(0, format_flags),
            },
        })
    }

    pub fn id(&self) -> &[u8; 4] {
        &self.frame_id
    }

    pub fn size(&self) -> usize {
        self.frame_size
    }

    pub fn flags(&self) -> &FrameFlags {
        &self.flags
    }

    pub fn id_str(&self) -> &str {
        str::from_utf8(self.id()).unwrap()
    }

    pub(crate) fn size_mut(&mut self) -> &mut usize {
        &mut self.frame_size
    }

    pub(crate) fn _id_mut(&mut self) -> &mut [u8; 4] {
        &mut self.frame_id
    }

    pub(crate) fn _flags_mut(&mut self) -> &mut FrameFlags {
        &mut self.flags
    }
}

pub struct FrameFlags {
    pub tag_alter_preservation: bool,
    pub file_alter_preservation: bool,
    pub read_only: bool,
    pub grouped: bool,
    pub compressed: bool,
    pub encrypted: bool,
    pub unsync: bool,
    pub data_len_indicator: bool,
}

impl Default for FrameFlags {
    fn default() -> Self {
        FrameFlags {
            tag_alter_preservation: false,
            file_alter_preservation: false,
            read_only: false,
            grouped: false,
            compressed: false,
            encrypted: false,
            unsync: false,
            data_len_indicator: false,
        }
    }
}

// --------
// This is where things get frustratingly messy. The ID3v2 spec tacks on so many things
// regarding frame headers that most of the instantiation code is horrific tangle of if
// blocks, sanity checks, and quirk workarounds to get a [mostly] working frame. Even this
// system however faces multiple downsides, but theres not alot we can do.
// --------

pub(crate) fn new(tag_header: &TagHeader, data: &[u8]) -> ParseResult<Box<dyn Frame>> {
    // Frame structure differs quite signifigantly across versions, so we have to
    // handle them seperately.

    match tag_header.major() {
        3 => parse_frame_v3(tag_header, data),
        4 => parse_frame_v4(tag_header, data),
        _ => Err(ParseError::Unsupported)
    }
}

pub(crate) fn parse_frame_v4(tag_header: &TagHeader, data: &[u8]) -> ParseResult<Box<dyn Frame>> {
    let mut frame_header = FrameHeader::parse_v4(data)?;

    // Ensure that we are in-bounds before continuing.
    if frame_header.size() == 0 || frame_header.size() > data.len() + 10 {
        return Err(ParseError::NotEnoughData);
    }
    
    // Validate our frame ID is valid.
    if !is_frame_id(frame_header.id()) {
        return Err(ParseError::InvalidData)
    }

    // ID3v2.4 sizes *should* be syncsafe, but iTunes wrote v2.3-style sizes for awhile. Fix that.
    if frame_header.size() >= 0x80 {
        let size = handle_itunes_v4_size(frame_header.size(), data);
        *frame_header.size_mut() = size
    }

    // To prevent needlessly copying the given data slice into a Vec, we keep a reference
    // of what data we will pass to the later parsing functions and modify it as needed,
    // replacing it with decoded data or modifying the starting position.
    // It's janky, but it generally works and is more efficent.

    let mut frame_data = &data[10..frame_header.size() + 10];
    let mut decoded_data = Vec::new();

    // Frame-specific unsynchronization. The spec is vague about whether the non-size bytes
    // are affected by unsynchronization, so we just assume that they are.
    if frame_header.flags().unsync || tag_header.flags().unsync {
        decoded_data = syncdata::decode(frame_data);
        frame_data = &decoded_data;
    }

    // Frame grouping. Is ignored.
    // TODO: Implement this if its used in the real world
    if frame_header.flags().grouped {
        frame_data = &frame_data[1..]
    }

    // Encryption. Will likely never be implemented since it's usually vendor-specific.
    if frame_header.flags().encrypted {
        return Ok(Box::new(UnknownFrame::with_data(frame_header, frame_data)));
    }

    // Data length indicator. Some taggers may not flip the data length indicator when
    // compression is enabled, so it's treated as implicitly enabling it.
    // The spec is vague about whether the length location is affected by the new flag
    // or the existing compression/encryption flags, so we just assume its the latter.
    // Not like it really matters since we always skip this size.
    if frame_header.flags().data_len_indicator || frame_header.flags().compressed {
        frame_data = &frame_data[4..];
    }

    // Frame-specific compression.
    if frame_header.flags().compressed {
        decoded_data = match inflate_frame(frame_data) {
            Ok(data) => data,
            Err(_) => return Ok(Box::new(UnknownFrame::with_data(frame_header, frame_data)))
        };
        
        frame_data = &decoded_data;
    }

    // Parse ID3v2.4-specific frames.
    let frame = match frame_header.id() {
        // Involved People List & Musician Credits List
        b"TIPL" | b"TMCL" => Box::new(CreditsFrame::parse(frame_header, frame_data)?),

        // TODO: Complete V4-specific frames
        // ASPI Audio seek point index
        // EQU2 Equalisation
        // RVA2 Relative volume adjustment
        // SEEK Seek frame
        // SIGN Signature frame

        _ => parse_frame(tag_header, frame_header, frame_data)?
    };

    let _ = decoded_data;

    Ok(frame)
}

pub(crate) fn parse_frame_v3(tag_header: &TagHeader, data: &[u8]) -> ParseResult<Box<dyn Frame>> {
    let frame_header = FrameHeader::parse_v3(data)?;

    // Ensure that we are in-bounds before continuing.
    if frame_header.size() == 0 || frame_header.size() > data.len() + 10 {
        return Err(ParseError::NotEnoughData);
    }

    let mut frame_data = &data[10..frame_header.size() + 10];
    let mut decoded_data: Vec<u8> = Vec::new();

    // iTunes writes ID3v2.3 frames with ID3v2 names. This error will be fixed eventually.
    if frame_header.id()[3] == 0 {
        return Err(ParseError::Unsupported)
    }

    // Validate our frame ID is valid.
    if !is_frame_id(frame_header.id()) {
        return Err(ParseError::InvalidData);
    }

    // Encryption. Will never be supported since its usually vendor-specific
    if frame_header.flags().encrypted {
        return Ok(Box::new(UnknownFrame::with_data(frame_header, data)));
    }

    // Frame-specific compression. This flag also adds a data length indicator that we will skip.
    if frame_header.flags().compressed {
        if frame_data.len() < 4 {
            return Err(ParseError::NotEnoughData);
        }

        decoded_data = match inflate_frame(&frame_data[4..]) {
            Ok(data) => data,
            Err(_) => return Ok(Box::new(UnknownFrame::with_data(frame_header, frame_data)))
        };
        
        frame_data = &decoded_data;
    }

    // Grouping identity, this time at the end since it's the last flag.
    if frame_header.flags().grouped && frame_data.len() >= 4 {
        frame_data = &frame_data[1..];
    }

    let frame = match frame_header.id() {
        // Involved People List
        b"IPLS" => Box::new(CreditsFrame::parse(frame_header, frame_data)?),

        // TODO: Complete V3-specific frames
        // RVAD: Relative volume adjustment
        // EQUA: Equalization [?]

        _ => parse_frame(tag_header, frame_header, frame_data)?
    };

    let _ = decoded_data;

    Ok(frame)
}

pub(crate) fn parse_frame(
    tag_header: &TagHeader,
    frame_header: FrameHeader,
    data: &[u8],    
) -> ParseResult<Box<dyn Frame>>  {
    // To parse most frames, we have to manually go through and determine what kind of
    // frame to create based on the frame id. There are many frame possibilities, so
    // there are many match arms.

    let frame: Box<dyn Frame> = match frame_header.id() {
        // Unique File Identifier [Frames 4.1]
        b"UFID" => Box::new(FileIdFrame::parse(frame_header, data)?),

        // --- Text Information [Frames 4.2] ---

        // User-Defined Text Information [Frames 4.2.6]
        b"TXXX" => Box::new(UserTextFrame::parse(frame_header, data)?),

        // Generic Text Information
        id if TextFrame::is_text(id) => Box::new(TextFrame::parse(frame_header, data)?),

        // --- URL Link [Frames 4.3] ---

        // User-Defined URL Link [Frames 4.3.2]
        b"WXXX" => Box::new(UserUrlFrame::parse(frame_header, data)?),

        // Generic URL Link
        id if id.starts_with(&[b'W']) => Box::new(UrlFrame::parse(frame_header, data)?),

        //  Music CD Identifier [Frames 4.4]
        b"MCDI" => todo!(),

        // Event timing codes [Frames 4.5]
        b"ETCO" => Box::new(EventTimingCodesFrame::parse(frame_header, data)?),

        // MPEG Lookup Codes [Frames 4.6]
        b"MLLT" => todo!(),

        // Synchronised tempo codes [Frames 4.7]
        b"SYTC" => todo!(),

        // Unsynchronized Lyrics [Frames 4.8]
        b"USLT" => Box::new(UnsyncLyricsFrame::parse(frame_header, data)?),

        // Unsynchronized Lyrics [Frames 4.9]
        b"SYLT" => Box::new(SyncedLyricsFrame::parse(frame_header, data)?),

        // Comments [Frames 4.10]
        b"COMM" => Box::new(CommentsFrame::parse(frame_header, data)?),

        // (Frames 4.11 & 4.12 are Verson-Specific)

        // Reverb [Frames 4.13]
        b"RVRB" => todo!(),

        // Attatched Picture [Frames 4.14]
        b"APIC" => Box::new(AttachedPictureFrame::parse(frame_header, data)?),

        // General Encapsulated Object [Frames 4.15]
        b"GEOB" => Box::new(GeneralObjectFrame::parse(frame_header, data)?),

        // Play Counter [Frames 4.16]
        b"PCNT" => Box::new(PlayCounterFrame::parse(frame_header, data)?),

        // Popularimeter [Frames 4.17]
        b"POPM" => Box::new(PopularimeterFrame::parse(frame_header, data)?),

        // Relative buffer size [Frames 4.18]
        b"RBUF" => todo!(),

        // Audio Encryption [Frames 4.19]
        b"AENC" => todo!(),

        // Linked Information [Frames 4.20]
        b"LINK" => todo!(),

        // Position synchronisation frame [Frames 4.21]
        b"POSS" => todo!(),

        // Terms of use frame [Frames 4.22]
        b"USER" => Box::new(TermsOfUseFrame::parse(frame_header, data)?),

        // Ownership frame [Frames 4.23]
        b"OWNE" => Box::new(OwnershipFrame::parse(frame_header, data)?),

        // Commercial frame [Frames 4.24]
        b"COMR" => todo!(),

        // Encryption Registration [Frames 4.25]
        b"ENCR" => todo!(),

        // Group Identification [Frames 4.26]
        b"GRID" => todo!(),

        // Private Frame [Frames 4.27]
        b"PRIV" => Box::new(PrivateFrame::parse(frame_header, data)?),

        // (Frames 4.28 -> 4.30 are version-specific)

        // Chapter Frame [ID3v2 Chapter Frame Addendum 3.1]
        b"CHAP" => Box::new(ChapterFrame::parse(frame_header, tag_header, data)?),

        // Table of Contents Frame [ID3v2 Chapter Frame Addendum 3.2]
        b"CTOC" => Box::new(TableOfContentsFrame::parse(frame_header, tag_header, data)?),
        
        // iTunes Podcast Frame
        b"PCST" => Box::new(PodcastFrame::parse(frame_header, data)?),

        // Unknown, return raw frame
        _ => Box::new(UnknownFrame::with_data(frame_header, data)),
    };

    Ok(frame)
}

fn handle_itunes_v4_size(sync_size: usize, data: &[u8]) -> usize {
    let next_id_start = sync_size + 10;
    let next_id_end = sync_size + 14;
    let next_id = next_id_start..next_id_end;

    // Ignore truncated data and padding
    if data.len() < next_id_end || data[next_id_start] == 0 {
        return sync_size;
    }

    if !is_frame_id(&data[next_id]) {
        // If the raw size leads us to the next frame where the "syncsafe"
        // size wouldn't, we will use that size instead.
        let raw_size = raw::to_size(&data[4..8]);

        if is_frame_id(&data[raw_size + 10..raw_size + 14]) {
            return raw_size;
        }
    }

    sync_size
}

#[cfg(feature = "id3v2_zlib")]
fn inflate_frame(data: &[u8]) -> ParseResult<Vec<u8>> {
    use miniz_oxide::inflate;

    inflate::decompress_to_vec_zlib(data)
        .map_err(|_| ParseError::InvalidData)
} 

#[cfg(not(feature = "id3v2_zlib"))]
fn inflate_frame(data: &[u8]) -> ParseResult<Vec<u8>> {
    Err(ParseError::Unsupported)
}

fn is_frame_id(frame_id: &[u8]) -> bool {
    for ch in frame_id {
        if !(b'A'..b'Z').contains(ch) && !(b'0'..b'9').contains(ch) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::File;
    use crate::id3v2::frames::AttachedPictureFrame;
    use crate::id3v2::frames::file::PictureType;
    use std::env;

    #[test]
    fn parse_v3_frame_header() {
        let data = b"TXXX\x00\x0A\x71\x7B\xA0\x40";
        let header = FrameHeader::parse_v3(&data[..]).unwrap();
        let flags = header.flags();

        assert_eq!(header.id(), b"TXXX");
        assert_eq!(header.size(), 684411);

        assert!(flags.tag_alter_preservation);
        assert!(!flags.file_alter_preservation);
        assert!(flags.read_only);

        assert!(!flags.compressed);
        assert!(flags.encrypted);
        assert!(!flags.grouped);
    }

    #[test]
    fn parse_v4_frame_header() {
        let data = b"TXXX\x00\x34\x10\x2A\x50\x4B";
        let header = FrameHeader::parse_v4(&data[..]).unwrap();
        let flags = header.flags();

        assert_eq!(header.id(), b"TXXX");
        assert_eq!(header.size(), 854058);

        assert!(flags.tag_alter_preservation);
        assert!(!flags.file_alter_preservation);
        assert!(flags.read_only);

        assert!(flags.grouped);
        assert!(flags.compressed);
        assert!(!flags.encrypted);
        assert!(flags.unsync);
        assert!(flags.data_len_indicator);
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

    #[test]
    fn parse_compressed_frames() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/compressed.mp3";
        let mut file = File::open(&path).unwrap();
        let tag = file.id3v2().unwrap();
        let apic = &tag.frames()["APIC:"].downcast::<AttachedPictureFrame>().unwrap();
        
        assert_eq!(apic.mime(), "image/bmp");
        assert_eq!(apic.pic_type(), PictureType::Other);
        assert_eq!(apic.desc(), "");
        assert_eq!(apic.picture().len(), 86414);
    }
}
