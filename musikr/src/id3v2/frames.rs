//! Frame parsing and implementations.
//!
//! An ID3v2 tag is primarily made up of chunks of data, called "Frames" by the spec.
//! These frames represent the metadata of the tag.
//! ID3v2 frames are largely structured as the following:
//!
//! ```text
//! [ID] [Size] [Encoding Flags] [Format Flags]
//! [Flag-specific header data]
//! [Frame body]
//! ```
//!
//! The header, [Represented as [`FrameHeader`](FrameHeader)] contains a unique 4-byte
//! identifier for the frame [Represented as [`FrameId`](FrameId)].
//!
//! This is then followed by further information about the size and formatting of the
//! frame. Frame flags [Represented by [`FrameFlags`](FrameFlags)] can also add information
//! to the frame header, however this has no analogue in musikr.
//!
//! The frame body tends to differ across frame types, providing the specific fields that
//! are exposed in frame implementations.
//!
//! One of the main ways that the ID3v2 module differs from the rest of musikr is that
//! frames are represented as a trait object. This is because frames tend to be extremely
//! heterogenous, making other solutions such as enums or a large struct either impractical
//! or prone to error. However, methods are supplied that help allieviate some of the problems
//! regarding trait objects.

pub mod audio;
pub mod bin;
pub mod chapters;
pub mod comments;
mod encoding;
pub mod events;
pub mod file;
pub mod lang;
pub mod lyrics;
pub mod owner;
pub mod stats;
pub mod text;
pub mod time;
pub mod url;

pub use audio::{EqualisationFrame2, RelativeVolumeFrame2};
pub use bin::{FileIdFrame, PodcastFrame, PrivateFrame, UnknownFrame};
pub use chapters::{ChapterFrame, TableOfContentsFrame};
pub use comments::CommentsFrame;
pub use events::EventTimingCodesFrame;
pub use file::{AttachedPictureFrame, GeneralObjectFrame};
pub use lyrics::{SyncedLyricsFrame, UnsyncLyricsFrame};
pub use owner::{OwnershipFrame, TermsOfUseFrame};
pub use stats::{PlayCounterFrame, PopularimeterFrame};
pub use text::{CreditsFrame, TextFrame, UserTextFrame};
pub use url::{UrlFrame, UserUrlFrame};

use crate::core::io::BufStream;
use crate::id3v2::tag::{TagHeader, Version};
use crate::id3v2::{syncdata, ParseError, ParseResult};

use dyn_clone::DynClone;
use std::any::Any;
use std::fmt::{self, Debug, Display, Formatter};
use std::str;

// TODO: Make tests use the main frames::new system.

pub trait Frame: Display + Debug + AsAny + DynClone {
    fn id(&self) -> &str {
        self.header().id().as_str()
    }

    fn key(&self) -> String;

    fn header(&self) -> &FrameHeader;
    fn header_mut(&mut self, _: Token) -> &mut FrameHeader;

    fn is_empty(&self) -> bool;
    fn render(&self, tag_header: &TagHeader) -> Vec<u8>;
}

impl dyn Frame {
    pub fn is<T: Frame>(&self) -> bool {
        self.as_any(Token::new()).is::<T>()
    }

    pub fn downcast<T: Frame>(&self) -> Option<&T> {
        self.as_any(Token::new()).downcast_ref::<T>()
    }

    pub fn downcast_mut<T: Frame>(&mut self) -> Option<&mut T> {
        self.as_any_mut(Token::new()).downcast_mut::<T>()
    }
}

pub trait AsAny: Any {
    fn as_any(&self, _: Token) -> &dyn Any;
    fn as_any_mut(&mut self, _: Token) -> &mut dyn Any;
}

impl<T: Frame> AsAny for T {
    fn as_any(&self, _: Token) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self, _: Token) -> &mut dyn Any {
        self
    }
}

dyn_clone::clone_trait_object!(Frame);

/// A token for calling internal methods.
///
/// Certain methods in this module are supposed to only be called by musikr,
/// such as [`Frame::header_mut`](Frame::header_mut), but still need to be
/// implemented by external users. This struct limits these methods by making
/// the only constructor private to the frames module.
pub struct Token(());

impl Token {
    fn new() -> Self {
        Self(())
    }
}

// --------
// This is where things get frustratingly messy. The ID3v2 spec tacks on so many things
// regarding frames that most of the instantiation and parsing code is horrific tangle
// of if blocks, sanity checks, and quirk workarounds to get a [mostly] working frame.
// You have been warned.
// --------

#[derive(Clone, Debug)]
pub struct FrameHeader {
    frame_id: FrameId,
    frame_size: usize,
    flags: FrameFlags,
}

impl FrameHeader {
    pub fn new(frame_id: FrameId) -> Self {
        FrameHeader {
            frame_id,
            frame_size: 0,
            flags: FrameFlags::default(),
        }
    }

    pub(crate) fn parse_v3(stream: &mut BufStream) -> ParseResult<Self> {
        // TODO: iTunes writes v2.2 frames to v2.3 tags. Handle that.
        let frame_id = FrameId::parse(&stream.read_array()?)?;

        let frame_size = stream.read_u32()? as usize;
        let stat_flags = stream.read_u8()?;
        let format_flags = stream.read_u8()?;

        Ok(Self {
            frame_id,
            frame_size,
            flags: FrameFlags {
                tag_alter_preservation: stat_flags & 0x80 != 0,
                file_alter_preservation: stat_flags & 0x40 != 0,
                read_only: stat_flags & 0x20 != 0,
                compressed: format_flags & 0x80 != 0,
                encrypted: format_flags & 0x40 != 0,
                grouped: format_flags & 0x20 != 0,
                ..Default::default()
            },
        })
    }

    pub(crate) fn parse_v4(stream: &mut BufStream) -> ParseResult<Self> {
        let frame_id = FrameId::parse(&stream.read_array()?)?;

        // ID3v2.4 sizes *should* be syncsafe, but iTunes wrote v2.3-style sizes for awhile. Fix that.
        let size_bytes = stream.read_array()?;
        let mut frame_size = syncdata::to_size(size_bytes);

        if frame_size >= 0x80 {
            // Theres a real possibility that this hack causes us to look out of bounds, so if
            // it fails we just use the normal size.
            frame_size = fix_itunes_frame_size(size_bytes, frame_size, stream).unwrap_or(frame_size)
        }

        let flags = stream.read_u16()?;

        Ok(Self {
            frame_id,
            frame_size,
            flags: FrameFlags {
                tag_alter_preservation: flags & 0x4000 != 0,
                file_alter_preservation: flags & 0x2000 != 0,
                read_only: flags & 0x1000 != 0,
                grouped: flags & 0x40 != 0,
                compressed: flags & 0x8 != 0,
                encrypted: flags & 0x4 != 0,
                unsync: flags & 0x2 != 0,
                data_len_indicator: flags & 0x1 != 0,
            },
        })
    }

    pub fn id(&self) -> &FrameId {
        &self.frame_id
    }

    pub fn size(&self) -> usize {
        self.frame_size
    }

    pub fn flags(&self) -> FrameFlags {
        self.flags
    }

    pub(crate) fn _id_mut(&mut self) -> &mut FrameId {
        &mut self.frame_id
    }

    pub(crate) fn _size_mut(&mut self) -> &mut usize {
        &mut self.frame_size
    }

    pub(crate) fn _flags_mut(&mut self) -> &mut FrameFlags {
        &mut self.flags
    }
}

fn fix_itunes_frame_size(
    size_bytes: [u8; 4],
    v4_size: usize,
    stream: &mut BufStream,
) -> ParseResult<usize> {
    let mut next_id = [0; 4];
    next_id.copy_from_slice(stream.peek(v4_size + 2..v4_size + 6)?);

    if next_id[0] != 0 && FrameId::parse(&next_id).is_err() {
        // If the raw size leads us to the next frame where the "syncsafe"
        // size wouldn't, we will use that size instead.
        let v3_size = u32::from_be_bytes(size_bytes) as usize;
        next_id.copy_from_slice(stream.peek(v3_size + 2..v3_size + 6)?);

        if FrameId::parse(&next_id).is_ok() {
            return Ok(v3_size);
        }
    }

    Ok(v4_size)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FrameId([u8; 4]);

impl FrameId {
    pub fn new(id: &[u8; 4]) -> Self {
        Self::parse(id).expect("Frame IDs must be 4 uppercase ASCII characters or numbers.")
    }

    pub fn parse(id: &[u8; 4]) -> ParseResult<Self> {
        for ch in id {
            // Valid frame IDs can only contain uppercase ASCII chars and numbers.
            if !(b'A'..b'Z').contains(ch) && !(b'0'..b'9').contains(ch) {
                return Err(ParseError::MalformedData);
            }
        }

        Ok(Self(*id))
    }

    pub fn inner(&self) -> &[u8; 4] {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        // We've asserted that this frame is ASCII, so we can unwrap.
        str::from_utf8(&self.0).unwrap()
    }

    pub fn starts_with(&self, ch: u8) -> bool {
        self.0[0] == ch
    }
}

impl Display for FrameId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.as_str()]
    }
}

impl PartialEq<[u8; 4]> for FrameId {
    fn eq(&self, other: &[u8; 4]) -> bool {
        self.0 == *other
    }
}

impl PartialEq<&[u8; 4]> for FrameId {
    fn eq(&self, other: &&[u8; 4]) -> bool {
        self == *other
    }
}

#[derive(Default, Clone, Copy, Debug)]
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

pub(crate) fn new(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<Box<dyn Frame>> {
    // Frame structure differs quite signifigantly across versions, so we have to
    // handle them seperately.

    match tag_header.version() {
        // TOOD: Add ID3v2.2 frames
        Version::V22 => Err(ParseError::Unsupported),
        Version::V23 => parse_frame_v3(tag_header, stream),
        Version::V24 => parse_frame_v4(tag_header, stream),
    }
}

fn parse_frame_v4(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<Box<dyn Frame>> {
    let frame_header = FrameHeader::parse_v4(stream)?;

    // As per the spec, empty frames should be treated as a sign of a malformed tag, meaning that
    // parsing should stop. This may change in the future.
    if frame_header.size() == 0 {
        return Err(ParseError::MalformedData);
    }

    // Keep track of both decoded data and a BufStream containing the frame data that will be used.
    // This seems a bit disjointed, but doing this allows us to avoid a needless copy of the original
    // stream into an owned stream just so that it would line up with any owned decoded streams.

    let mut stream = stream.slice_stream(frame_header.size())?;
    let mut decoded = Vec::new();

    // Frame-specific unsynchronization. The spec is vague about whether the non-size bytes
    // are affected by unsynchronization, so we just assume that they are.
    if frame_header.flags().unsync || tag_header.flags().unsync {
        decoded = syncdata::decode(&mut stream);
        stream = BufStream::new(&decoded);
    }

    // Frame grouping. Is ignored.
    // TODO: Implement this if its used in the real world
    if frame_header.flags().grouped {
        stream.skip(1)?;
    }

    // Encryption. Will likely never be implemented since it's usually vendor-specific.
    if frame_header.flags().encrypted {
        return Ok(Box::new(UnknownFrame::from_stream(
            frame_header,
            &mut stream,
        )));
    }

    // Data length indicator. Some taggers may not flip the data length indicator when
    // compression is enabled, so it's treated as implicitly enabling it.
    // The spec is also vague about whether the length location is affected by the new flag
    // or the existing compression/encryption flags, so we just assume its the latter.
    // Not like it really matters since we always skip this.
    if frame_header.flags().data_len_indicator || frame_header.flags().compressed {
        stream.skip(4)?;
    }

    // Frame-specific compression.
    if frame_header.flags().compressed {
        decoded = match inflate_stream(&mut stream) {
            Ok(stream) => stream,
            Err(_) => {
                return Ok(Box::new(UnknownFrame::from_stream(
                    frame_header,
                    &mut stream,
                )))
            }
        };

        stream = BufStream::new(&decoded);
    }

    // Parse ID3v2.4-specific frames.
    let frame: Box<dyn Frame> = match frame_header.id().inner() {
        // Involved People List & Musician Credits List [Frames 4.2.2]
        b"TIPL" | b"TMCL" => Box::new(CreditsFrame::parse(frame_header, &mut stream)?),

        // Relative Volume Adjustment 2 [Frames 4.11]
        b"RVA2" => Box::new(RelativeVolumeFrame2::parse(frame_header, &mut stream)?),

        // Equalisation 2 [Frames 4.12]
        b"EQU2" => Box::new(EqualisationFrame2::parse(frame_header, &mut stream)?),

        // Signature Frame [Frames 4.28]
        // b"SIGN" => todo!(),

        // Seek frame [Frames 4.27]
        // b"SEEK" => todo!(),

        // Audio seek point index [Frames 4.30]
        // b"ASPI" => todo!(),
        _ => parse_frame(tag_header, frame_header, &mut stream)?,
    };

    let _ = decoded;

    Ok(frame)
}

fn parse_frame_v3(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<Box<dyn Frame>> {
    let frame_header = FrameHeader::parse_v3(stream)?;

    // As per the spec, empty frames should be treated as a sign of a malformed tag, meaning that
    // parsing should stop. This may change in the future.
    if frame_header.size() == 0 {
        return Err(ParseError::MalformedData);
    }

    // Keep track of both decoded data and a BufStream containing the frame data that will be used.
    // This seems a bit disjointed, but doing this allows us to avoid a needless copy of the original
    // stream into an owned stream just so that it would line up with any owned decoded streams.

    let mut stream = stream.slice_stream(frame_header.size())?;
    let mut decoded = Vec::new();

    // Encryption. Will never be supported since its usually vendor-specific
    if frame_header.flags().encrypted {
        return Ok(Box::new(UnknownFrame::from_stream(
            frame_header,
            &mut stream,
        )));
    }

    // Frame-specific compression. This flag also adds a data length indicator that we will skip.
    if frame_header.flags().compressed {
        stream.skip(4)?;

        decoded = match inflate_stream(&mut stream) {
            Ok(stream) => stream,
            Err(_) => {
                return Ok(Box::new(UnknownFrame::from_stream(
                    frame_header,
                    &mut stream,
                )))
            }
        };

        stream = BufStream::new(&decoded);
    }

    // Grouping identity, this time at the end since it's the last flag.
    if frame_header.flags().grouped && stream.len() >= 4 {
        stream.skip(1)?;
    }

    // Match ID3v2.3-specific frames
    let frame = match frame_header.id().inner() {
        // Involved People List
        b"IPLS" => Box::new(CreditsFrame::parse(frame_header, &mut stream)?),

        // Relative volume adjustment [Frames 4.12]
        // b"RVAD" => todo!(),

        // Equalisation [Frames 4.13]
        // b"EQUA" => todo!(),
        _ => parse_frame(tag_header, frame_header, &mut stream)?,
    };

    let _ = decoded;

    Ok(frame)
}

pub(crate) fn parse_frame(
    tag_header: &TagHeader,
    frame_header: FrameHeader,
    stream: &mut BufStream,
) -> ParseResult<Box<dyn Frame>> {
    // To parse most frames, we have to manually go through and determine what kind of
    // frame to create based on the frame id. There are many frame possibilities, so
    // there are many match arms.

    let frame_id = *frame_header.id();

    let frame: Box<dyn Frame> = match frame_id.inner() {
        // Unique File Identifier [Frames 4.1]
        b"UFID" => Box::new(FileIdFrame::parse(frame_header, stream)?),

        // --- Text Information [Frames 4.2] ---

        // User-Defined Text Information [Frames 4.2.6]
        b"TXXX" => Box::new(UserTextFrame::parse(frame_header, stream)?),

        // Generic Text Information
        _ if TextFrame::is_text(frame_id) => Box::new(TextFrame::parse(frame_header, stream)?),

        // --- URL Link [Frames 4.3] ---

        // User-Defined URL Link [Frames 4.3.2]
        b"WXXX" => Box::new(UserUrlFrame::parse(frame_header, stream)?),

        // Generic URL Link
        _ if frame_id.starts_with(b'W') => Box::new(UrlFrame::parse(frame_header, stream)?),

        // Music CD Identifier [Frames 4.4]
        // b"MCDI" => todo!(),

        // Event timing codes [Frames 4.5]
        b"ETCO" => Box::new(EventTimingCodesFrame::parse(frame_header, stream)?),

        // MPEG Lookup Codes [Frames 4.6]
        // b"MLLT" => todo!(),

        // Synchronised tempo codes [Frames 4.7]
        // b"SYTC" => todo!(),

        // Unsynchronized Lyrics [Frames 4.8]
        b"USLT" => Box::new(UnsyncLyricsFrame::parse(frame_header, stream)?),

        // Unsynchronized Lyrics [Frames 4.9]
        b"SYLT" => Box::new(SyncedLyricsFrame::parse(frame_header, stream)?),

        // Comments [Frames 4.10]
        b"COMM" => Box::new(CommentsFrame::parse(frame_header, stream)?),

        // (Frames 4.11 & 4.12 are Verson-Specific)

        // Reverb [Frames 4.13]
        // b"RVRB" => todo!(),

        // Attatched Picture [Frames 4.14]
        b"APIC" => Box::new(AttachedPictureFrame::parse(frame_header, stream)?),

        // General Encapsulated Object [Frames 4.15]
        b"GEOB" => Box::new(GeneralObjectFrame::parse(frame_header, stream)?),

        // Play Counter [Frames 4.16]
        b"PCNT" => Box::new(PlayCounterFrame::parse(frame_header, stream)?),

        // Popularimeter [Frames 4.17]
        b"POPM" => Box::new(PopularimeterFrame::parse(frame_header, stream)?),

        // Relative buffer size [Frames 4.18]
        // b"RBUF" => todo!(),

        // Audio Encryption [Frames 4.19]
        // b"AENC" => todo!(),

        // Linked Information [Frames 4.20]
        // b"LINK" => todo!(),

        // Position synchronisation frame [Frames 4.21]
        // b"POSS" => todo!(),

        // Terms of use frame [Frames 4.22]
        b"USER" => Box::new(TermsOfUseFrame::parse(frame_header, stream)?),

        // Ownership frame [Frames 4.23]
        b"OWNE" => Box::new(OwnershipFrame::parse(frame_header, stream)?),

        // Commercial frame [Frames 4.24]
        // b"COMR" => todo!(),

        // Encryption Registration [Frames 4.25]
        // b"ENCR" => todo!(),

        // Group Identification [Frames 4.26]
        // b"GRID" => todo!(),

        // Private Frame [Frames 4.27]
        b"PRIV" => Box::new(PrivateFrame::parse(frame_header, stream)?),

        // (Frames 4.28 -> 4.30 are version-specific)

        // Chapter Frame [ID3v2 Chapter Frame Addendum 3.1]
        b"CHAP" => Box::new(ChapterFrame::parse(frame_header, tag_header, stream)?),

        // Table of Contents Frame [ID3v2 Chapter Frame Addendum 3.2]
        b"CTOC" => Box::new(TableOfContentsFrame::parse(
            frame_header,
            tag_header,
            stream,
        )?),

        // iTunes Podcast Frame
        b"PCST" => Box::new(PodcastFrame::parse(frame_header, stream)?),

        // No idea, return unknown frame
        _ => Box::new(UnknownFrame::from_stream(frame_header, stream)),
    };

    Ok(frame)
}

#[cfg(feature = "id3v2_zlib")]
fn inflate_stream(src: &mut BufStream) -> ParseResult<Vec<u8>> {
    use miniz_oxide::inflate;

    inflate::decompress_to_vec_zlib(src.take_rest()).map_err(|_| ParseError::MalformedData)
}

#[cfg(not(feature = "id3v2_zlib"))]
fn inflate_stream(data: &mut BufStream) -> ParseResult<Vec<u8>> {
    Err(ParseError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::frames::file::PictureType;
    use crate::id3v2::frames::AttachedPictureFrame;
    use crate::id3v2::Tag;
    use std::env;

    #[test]
    fn parse_v3_frame_header() {
        let data = b"TXXX\x00\x0A\x71\x7B\xA0\x40";
        let header = FrameHeader::parse_v3(&mut BufStream::new(&data[..])).unwrap();
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
        let header = FrameHeader::parse_v4(&mut BufStream::new(&data[..])).unwrap();
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
        let tag = Tag::open(&path).unwrap();

        assert_eq!(tag.frames["TIT2"].to_string(), "Sunshine Superman");
        assert_eq!(tag.frames["TPE1"].to_string(), "Donovan");
        assert_eq!(tag.frames["TALB"].to_string(), "Sunshine Superman");
        assert_eq!(tag.frames["TRCK"].to_string(), "1");
    }

    #[test]
    fn parse_compressed_frames() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/compressed.mp3";
        let tag = Tag::open(&path).unwrap();
        let apic = &tag.frames["APIC:"]
            .downcast::<AttachedPictureFrame>()
            .unwrap();

        assert_eq!(apic.mime, "image/bmp");
        assert_eq!(apic.pic_type, PictureType::Other);
        assert_eq!(apic.desc, "");
        assert_eq!(apic.picture.len(), 86414);
    }
}
