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

pub use audio::{RelativeVolumeFrame2, EqualisationFrame2};
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
use crate::id3v2::{syncdata, ParseError, ParseResult, TagHeader};

use std::any::Any;
use std::convert::TryInto;
use std::fmt::Display;
use std::str;

// TODO: Make tests use the main frames::new system.

// The id3v2::Frame downcasting system is derived from downcast-rs.
// https://github.com/marcianx/downcast-rs

pub trait Frame: Display + AsAny {
    fn id(&self) -> &str {
        self.header().id_str()
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

pub struct Token {
    _inner: (),
}

impl Token {
    fn _new() -> Self {
        Token { _inner: () }
    }
}

pub struct FrameHeader {
    frame_id: [u8; 4],
    frame_size: usize,
    flags: FrameFlags,
}

impl FrameHeader {
    pub fn new(frame_id: &[u8; 4]) -> Self {
        if !is_frame_id(frame_id) {
            // It's generally better to panic here as passing a malformed ID is usually programmer error.
            panic!("A Frame ID must be exactly four valid uppercase ASCII characters or numbers.")
        }

        FrameHeader {
            frame_id: *frame_id,
            frame_size: 0,
            flags: FrameFlags::default(),
        }
    }

    pub(crate) fn parse_v3(stream: &mut BufStream) -> ParseResult<Self> {
        let frame_id = stream.read_array::<4>()?;
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
        let frame_id = stream.read_array::<4>()?;
        let frame_size = syncdata::read_size(stream)?;

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
        // We've garunteed the ID is pure-ASCII, so we can unwrap.
        str::from_utf8(self.id()).unwrap()
    }

    pub(crate) fn _id_mut(&mut self) -> &mut [u8; 4] {
        &mut self.frame_id
    }

    pub(crate) fn size_mut(&mut self) -> &mut usize {
        &mut self.frame_size
    }

    pub(crate) fn _flags_mut(&mut self) -> &mut FrameFlags {
        &mut self.flags
    }
}

#[derive(Default)]
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

// --------
// This is where things get frustratingly messy. The ID3v2 spec tacks on so many things
// regarding frame headers that most of the instantiation code is horrific tangle of if
// blocks, sanity checks, and quirk workarounds to get a [mostly] working frame. You have
// been warned.
// --------

pub(crate) fn new(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<Box<dyn Frame>> {
    // Frame structure differs quite signifigantly across versions, so we have to
    // handle them seperately.

    match tag_header.major() {
        3 => parse_frame_v3(tag_header, stream),
        4 => parse_frame_v4(tag_header, stream),
        _ => Err(ParseError::Unsupported),
    }
}

fn parse_frame_v4(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<Box<dyn Frame>> {
    let mut frame_header = FrameHeader::parse_v4(stream)?;

    // Validate our frame id. If this is invalid, then its assumed padding was reached and the
    // parsing will stop.
    if !is_frame_id(frame_header.id()) {
        return Err(ParseError::MalformedData);
    }

    // ID3v2.4 sizes *should* be syncsafe, but iTunes wrote v2.3-style sizes for awhile. Fix that.
    if frame_header.size() >= 0x80 {
        let size = handle_itunes_v4_size(frame_header.size(), stream)
            .unwrap_or_else(|_| frame_header.size());

        *frame_header.size_mut() = size
    }

    // As per the spec, empty frames should be treated as a sign of a malformed tag, meaning that
    // parsing should stop. This may change in the future.
    if frame_header.size() == 0 {
        return Err(ParseError::MalformedData)
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
    let frame: Box<dyn Frame> = match frame_header.id() {
        // Involved People List & Musician Credits List [Frames 4.2.2]
        b"TIPL" | b"TMCL" => Box::new(CreditsFrame::parse(frame_header, &mut stream)?),

        // Relative Volume Adjustment 2 [Frames 4.11]
        b"RVA2" => Box::new(RelativeVolumeFrame2::parse(frame_header, &mut stream)?),

        // Equalisation 2 [Frames 4.12]
        b"EQU2" => Box::new(EqualisationFrame2::parse(frame_header, &mut stream)?),

        // Signature Frame [Frames 4.28]
        b"SIGN" => todo!(),

        // Seek frame [Frames 4.27]
        b"SEEK" => todo!(),

        // Audio seek point index [Frames 4.30]
        b"ASPI" => todo!(),

        _ => parse_frame(tag_header, frame_header, &mut stream)?,
    };

    let _ = decoded;

    Ok(frame)
}

fn parse_frame_v3(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<Box<dyn Frame>> {
    let frame_header = FrameHeader::parse_v3(stream)?;

    // iTunes writes ID3v2.3 frames with ID3v2 names. This error will be fixed eventually.
    if frame_header.id()[3] == 0 {
        return Err(ParseError::Unsupported);
    }

    // Validate our frame id. If this is invalid, then its assumed padding was reached and the
    // parsing will stop.
    if !is_frame_id(frame_header.id()) {
        return Err(ParseError::MalformedData);
    }

    // As per the spec, empty frames should be treated as a sign of a malformed tag, meaning that
    // parsing should stop. This may change in the future.
    if frame_header.size() == 0 {
        return Err(ParseError::MalformedData)
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

    // Match V3-specific frames
    let frame = match frame_header.id() {
        // Involved People List
        b"IPLS" => Box::new(CreditsFrame::parse(frame_header, &mut stream)?),

        // Relative volume adjustment [Frames 4.12]
        b"RVAD" => todo!(),

        // Equalisation [Frames 4.13]
        b"EQUA" => todo!(),

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

    let frame: Box<dyn Frame> = match frame_header.id() {
        // Unique File Identifier [Frames 4.1]
        b"UFID" => Box::new(FileIdFrame::parse(frame_header, stream)?),

        // --- Text Information [Frames 4.2] ---

        // User-Defined Text Information [Frames 4.2.6]
        b"TXXX" => Box::new(UserTextFrame::parse(frame_header, stream)?),

        // Generic Text Information
        id if TextFrame::is_text(id) => Box::new(TextFrame::parse(frame_header, stream)?),

        // --- URL Link [Frames 4.3] ---

        // User-Defined URL Link [Frames 4.3.2]
        b"WXXX" => Box::new(UserUrlFrame::parse(frame_header, stream)?),

        // Generic URL Link
        id if id.starts_with(&[b'W']) => Box::new(UrlFrame::parse(frame_header, stream)?),

        // Music CD Identifier [Frames 4.4]
        b"MCDI" => todo!(),

        // Event timing codes [Frames 4.5]
        b"ETCO" => Box::new(EventTimingCodesFrame::parse(frame_header, stream)?),

        // MPEG Lookup Codes [Frames 4.6]
        b"MLLT" => todo!(),

        // Synchronised tempo codes [Frames 4.7]
        b"SYTC" => todo!(),

        // Unsynchronized Lyrics [Frames 4.8]
        b"USLT" => Box::new(UnsyncLyricsFrame::parse(frame_header, stream)?),

        // Unsynchronized Lyrics [Frames 4.9]
        b"SYLT" => Box::new(SyncedLyricsFrame::parse(frame_header, stream)?),

        // Comments [Frames 4.10]
        b"COMM" => Box::new(CommentsFrame::parse(frame_header, stream)?),

        // (Frames 4.11 & 4.12 are Verson-Specific)

        // Reverb [Frames 4.13]
        b"RVRB" => todo!(),

        // Attatched Picture [Frames 4.14]
        b"APIC" => Box::new(AttachedPictureFrame::parse(frame_header, stream)?),

        // General Encapsulated Object [Frames 4.15]
        b"GEOB" => Box::new(GeneralObjectFrame::parse(frame_header, stream)?),

        // Play Counter [Frames 4.16]
        b"PCNT" => Box::new(PlayCounterFrame::parse(frame_header, stream)?),

        // Popularimeter [Frames 4.17]
        b"POPM" => Box::new(PopularimeterFrame::parse(frame_header, stream)?),

        // Relative buffer size [Frames 4.18]
        b"RBUF" => todo!(),

        // Audio Encryption [Frames 4.19]
        b"AENC" => todo!(),

        // Linked Information [Frames 4.20]
        b"LINK" => todo!(),

        // Position synchronisation frame [Frames 4.21]
        b"POSS" => todo!(),

        // Terms of use frame [Frames 4.22]
        b"USER" => Box::new(TermsOfUseFrame::parse(frame_header, stream)?),

        // Ownership frame [Frames 4.23]
        b"OWNE" => Box::new(OwnershipFrame::parse(frame_header, stream)?),

        // Commercial frame [Frames 4.24]
        b"COMR" => todo!(),

        // Encryption Registration [Frames 4.25]
        b"ENCR" => todo!(),

        // Group Identification [Frames 4.26]
        b"GRID" => todo!(),

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

        // Unknown, return unknown frame
        _ => Box::new(UnknownFrame::from_stream(frame_header, stream)),
    };

    Ok(frame)
}

fn handle_itunes_v4_size(sync_size: usize, stream: &mut BufStream) -> ParseResult<usize> {
    let next_id_start = sync_size + 10;
    let next_id_end = sync_size + 14;

    let next_id = stream.peek(next_id_start..next_id_end)?;

    if next_id[0] != 0 && !is_frame_id(next_id) {
        // If the raw size leads us to the next frame where the "syncsafe"
        // size wouldn't, we will use that size instead.

        let v3_size = u32::from_be_bytes(
            // Ensured to be 4 bytes, so we can unwrap
            stream
                .peek(next_id_start + 4..next_id_end + 4)?
                .try_into()
                .unwrap(),
        ) as usize;

        if is_frame_id(stream.peek(v3_size + 10..v3_size + 14)?) {
            return Ok(v3_size);
        }
    }

    Ok(sync_size)
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
        let frames = tag.frames();

        assert_eq!(frames["TIT2"].to_string(), "Sunshine Superman");
        assert_eq!(frames["TPE1"].to_string(), "Donovan");
        assert_eq!(frames["TALB"].to_string(), "Sunshine Superman");
        assert_eq!(frames["TRCK"].to_string(), "1");
    }

    #[test]
    fn parse_compressed_frames() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/compressed.mp3";
        let tag = Tag::open(&path).unwrap();
        let apic = &tag.frames()["APIC:"]
            .downcast::<AttachedPictureFrame>()
            .unwrap();

        assert_eq!(apic.mime, "image/bmp");
        assert_eq!(apic.pic_type, PictureType::Other);
        assert_eq!(apic.desc, "");
        assert_eq!(apic.picture.len(), 86414);
    }
}
