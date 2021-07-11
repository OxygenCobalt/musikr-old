//! Frame parsing and implementations.
//!
//! An ID3v2 tag is primarily made up of chunks of data, called "Frames" by the spec.
//! Frames are highly structured and can contain a variety of information about the audio,
//! including audio adjustments and binary data.
//!
//! One of the main ways that the ID3v2 module differs from the rest of musikr is that
//! frames are represented as a trait object. This is because frames tend to be extremely
//! heterogenous, making other solutions such as enums or a large struct either impractical
//! or prone to error. However, methods are supplied that help allieviate some of the problems
//! regarding trait objects.

pub mod audio_v3;
pub mod audio_v4;
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

pub use audio_v3::{EqualisationFrame, RelativeVolumeFrame};
pub use audio_v4::{EqualisationFrame2, RelativeVolumeFrame2};
pub use bin::{FileIdFrame, PodcastFrame, PrivateFrame};
pub use chapters::{ChapterFrame, TableOfContentsFrame};
pub use comments::CommentsFrame;
pub use events::EventTimingCodesFrame;
pub use file::{AttachedPictureFrame, GeneralObjectFrame};
pub use lyrics::{SyncedLyricsFrame, UnsyncLyricsFrame};
pub use owner::{CommercialFrame, OwnershipFrame, TermsOfUseFrame};
pub use stats::{PlayCounterFrame, PopularimeterFrame};
pub use text::{CreditsFrame, TextFrame, UserTextFrame};
pub use url::{UrlFrame, UserUrlFrame};

use crate::core::io::BufStream;
use crate::id3v2::tag::{TagHeader, Version};
use crate::id3v2::{compat, syncdata, ParseError, ParseResult, SaveError, SaveResult};

use dyn_clone::DynClone;
use log::{error, info, warn};
use std::any::Any;
use std::convert::TryInto;
use std::fmt::{self, Debug, Display, Formatter};
use std::str::{self, FromStr};

pub trait Frame: Display + Debug + AsAny + DynClone {
    fn id(&self) -> FrameId;
    fn key(&self) -> String;
    fn is_empty(&self) -> bool;
    fn render(&self, tag_header: &TagHeader) -> Vec<u8>;
}

impl dyn Frame {
    pub fn is<T: Frame>(&self) -> bool {
        self.as_any(Sealed(())).is::<T>()
    }

    pub fn downcast<T: Frame>(&self) -> Option<&T> {
        self.as_any(Sealed(())).downcast_ref::<T>()
    }

    pub fn downcast_mut<T: Frame>(&mut self) -> Option<&mut T> {
        self.as_any_mut(Sealed(())).downcast_mut::<T>()
    }
}

pub trait AsAny: Any {
    fn as_any(&self, _: Sealed) -> &dyn Any;
    fn as_any_mut(&mut self, _: Sealed) -> &mut dyn Any;
}

impl<T: Frame> AsAny for T {
    fn as_any(&self, _: Sealed) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self, _: Sealed) -> &mut dyn Any {
        self
    }
}

dyn_clone::clone_trait_object!(Frame);

/// A token for limiting internal methods that are required to be public.
pub struct Sealed(());

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FrameId([u8; 4]);

impl FrameId {
    pub fn new(id: &[u8; 4]) -> Self {
        Self::parse(id).expect("invalid frame id: can only be uppercase ASCII chars")
    }

    pub fn parse(frame_id: &[u8; 4]) -> ParseResult<Self> {
        if !Self::is_valid(frame_id) {
            return Err(ParseError::MalformedData);
        }

        Ok(Self(*frame_id))
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

    fn is_valid(frame_id: &[u8]) -> bool {
        for ch in frame_id {
            // Valid frame IDs can only contain uppercase ASCII chars and numbers.
            if !(b'A'..=b'Z').contains(ch) && !(b'0'..=b'9').contains(ch) {
                return false;
            }
        }

        true
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

impl AsRef<[u8]> for FrameId {
    fn as_ref(&self) -> &'_ [u8] {
        &self.0
    }
}

impl FromStr for FrameId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut id = [0; 4];

        if s.len() != 4 {
            return Err(ParseError::MalformedData);
        }

        for (i, ch) in s.chars().enumerate() {
            if !ch.is_ascii() {
                return Err(ParseError::MalformedData);
            }

            id[i] = ch as u8;
        }

        FrameId::parse(&id)
    }
}

/// A frame that could not be fully decoded
/// 
/// Musikr cannot decode certain frames, such as encrypted frames or ID3v2.2 frames
/// that have no ID3v2.3 analogue. If this is the case, then this struct is returned.
/// UnknownFrame instances are immutable and are dropped when a tag is upgraded.
/// 
/// An UnknownFrame is **not** a [`Frame`](Frame). They can violate certain invariants and cannot be added
/// to a [`FrameMap`](crate::id3v2::collections::FrameMap). 
/// 
/// Generally, these invariants are garunteed:
/// - The Frame ID is proper ASCII characters and numbers
/// - The frame body is unsynchronized
/// 
/// These invariants cannot be garunteed:
/// - The frame has been fully decompressed
/// - The body has all it's auxillary data [such as a data length indicator] skipped
/// - The frame will be parsable, even if fully decoded
/// 
#[derive(Clone, Debug)]
pub struct UnknownFrame {
    frame_id: Vec<u8>,
    flags: u16,
    data: Vec<u8>
}

impl UnknownFrame {
    fn new<S: AsRef<[u8]>>(frame_id: S, flags: u16, stream: &BufStream) -> Self {
        UnknownFrame {
            frame_id: frame_id.as_ref().to_vec(),
            flags,
            data: stream.to_vec()
        }
    }

    /// Returns the ID of this tag. This will be a valid frame ID, but may be 3 bytes
    /// or 4 bytes depending on the tag version.
    pub fn id(&self) -> &[u8] {
        &self.frame_id
    }

    /// Returns the two flag bytes of this frame. This can be used as a guide for further
    /// parsing the frame.
    pub fn flags(&self) -> u16 {
        self.flags
    }

    /// Returns the data of the frame. This will include the entire frame body, including
    /// data length indicators and other auxillary data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub(crate) fn id_str(&self) -> &str {
        str::from_utf8(&self.frame_id).unwrap()
    }
}

// --------
// This is where things get frustratingly messy. The ID3v2 spec tacks on so many things
// regarding frames that most of the instantiation and parsing code is a horrific tangle
// of if blocks, sanity checks, and quirk workarounds to get a [mostly] working frame.
// Theres a reason why we dont include the frame header with frame instances.
// You have been warned.
// --------

#[derive(Debug)]
pub(crate) enum FrameResult {
    Frame(Box<dyn Frame>),
    Unknown(UnknownFrame),
    Dropped,
}

// Internal macro for quickly generating a FrameResult

macro_rules! frame {
    ($frame:expr) => {
        FrameResult::Frame(Box::new($frame))
    };
}

pub(crate) fn parse(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<FrameResult> {
    // Frame structure differs quite signifigantly across versions, so we have to
    // handle them seperately.

    match tag_header.version() {
        Version::V22 => parse_frame_v2(tag_header, stream),
        Version::V23 => parse_frame_v3(tag_header, stream),
        Version::V24 => parse_frame_v4(tag_header, stream),
    }
}

fn parse_frame_v2(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<FrameResult> {
    // ID3v2.2 frames are a 3-byte identifier and a 3-byte big-endian size.
    let frame_id = stream.read_array::<3>()?;

    if !FrameId::is_valid(&frame_id) {
        return Err(ParseError::MalformedData);
    }

    // Make u32::from_be_bytes handle the weird 3-byte sizes
    let mut size_bytes = [0; 4];
    size_bytes[1..4].copy_from_slice(&stream.read_array::<3>()?);
    let size = u32::from_be_bytes(size_bytes) as usize;

    // Luckily for us, we dont need to do any decoding magic for ID3v2.2 frames.
    let mut stream = stream.slice_stream(size)?;

    match_frame_v2(tag_header, &frame_id, &mut stream)
}

fn parse_frame_v3(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<FrameResult> {
    // iTunes writes ID3v2.2 frames to ID3v2.3 tags. Fix that.
    let id_bytes = stream.read_array()?;
    let size = stream.read_u32()? as usize;
    let flags = stream.read_u16()?;

    // Technically, the spec says that empty frames should be a sign of a malformed tag, but theyre
    // so common to the point where we should just skip them so other frames can be found.
    if size == 0 {
        return Ok(FrameResult::Dropped);
    }

    // Keep track of both decoded data and a BufStream containing the frame data that will be used.
    // This seems a bit disjointed, but doing this allows us to avoid a needless copy of the original
    // stream into an owned stream just so that it would line up with any owned decoded streams.

    let mut stream = stream.slice_stream(size)?;
    #[allow(unused_assignments)]
    let mut decoded = Vec::new();

    // Certain taggers will write ID3v2.2 frames to ID3v2.3 frames.
    // Since this hack is fallible, we need to do it after the stream is sliced so we can return
    // an unknown frame if it fails.
    let frame_id = match FrameId::parse(&id_bytes) {
        Ok(id) => id,
        Err(err) => {
            if FrameId::is_valid(&id_bytes[0..3]) && id_bytes[3] == 0 {
                info!("correcting incorrect ID3v2.2 frame ID");

                let mut v2_id = [0; 3];
                v2_id.copy_from_slice(&id_bytes[0..3]);

                return match_frame_v2(tag_header, &v2_id, &mut stream);
            }

            return Err(err);
        }
    };

    // Encryption. This really can't be implemented since:
    // A. Encryption is vendor-specific
    // B. Even if we *were* to add a Fn-pointer for the end-user handle these frames, the end-user
    // still doesn't know what kind of encryption the frame might have since the corresponding ENCR
    // frame might have not even been parsed yet.
    //
    // The way encryption is designed in ID3v2.3 and ID3v2.4 is absolutely busted, and honestly it
    // would be so much better if a metaframe like ID3v2.2's CRM was used instead. Oh well.
    if flags & 0x40 != 0 {
        warn!("encryption is not supported");
        return Ok(FrameResult::Unknown(
            UnknownFrame::new(frame_id, flags, &stream)
        ));
    }

    // Frame-specific compression. This flag also adds a data length indicator that we will skip.
    if flags & 0x80 != 0 {
        stream.skip(4)?;

        decoded = match inflate_frame(&mut stream) {
            Ok(stream) => stream,
            Err(_) => return Ok(FrameResult::Unknown(
                UnknownFrame::new(frame_id, flags, &stream)
            ))
        };

        stream = BufStream::new(&decoded);
    }

    // Frame grouping. Pretty much nobody uses this, so its ignored.
    if flags & 0x20 != 0 && stream.len() >= 4 {
        stream.skip(1)?;
    }

    match_frame_v3(tag_header, frame_id, &mut stream)
}

fn parse_frame_v4(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<FrameResult> {
    let frame_id = FrameId::parse(&stream.read_array()?)?;

    // ID3v2.4 sizes *should* be syncsafe, but iTunes wrote v2.3-style sizes for awhile. Fix that.
    let size_bytes = stream.read_array()?;
    let mut size = syncdata::to_u28(size_bytes) as usize;

    if size >= 0x80 {
        let mut next_id = [0; 4];

        if let Ok(id) = stream.peek(size + 2..size + 6) {
            next_id.copy_from_slice(id)
        }

        if next_id[0] != 0 && !FrameId::is_valid(&next_id) {
            // If the raw size leads us to the next frame where the "syncsafe"
            // size wouldn't, we will use that size instead.
            let v3_size = u32::from_be_bytes(size_bytes) as usize;

            if let Ok(id) = stream.peek(v3_size + 2..v3_size + 6) {
                next_id.copy_from_slice(id)
            }

            if FrameId::is_valid(&next_id) {
                info!("correcting non-syncsafe ID3v2.4 frame size");
                size = v3_size;
            }
        }
    }

    let flags = stream.read_u16()?;

    // Technically, the spec says that empty frames should be a sign of a malformed tag, but theyre
    // so common to the point where we should just skip them so other frames can be found.
    if size == 0 {
        return Ok(FrameResult::Dropped);
    }

    // Keep track of both decoded data and a BufStream containing the frame data that will be used.
    // This seems a bit disjointed, but doing this allows us to avoid a needless copy of the original
    // stream into an owned stream just so that it would line up with any owned decoded streams.

    let mut stream = stream.slice_stream(size as usize)?;
    #[allow(unused_assignments)]
    let mut decoded = Vec::new();

    // Frame-specific unsynchronization. The spec is vague about whether the non-size bytes
    // are affected by unsynchronization, so we just assume that they are.
    if flags & 0x2 != 0 || tag_header.flags().unsync {
        // Make sure this flag is cleared afterwards, as this frame might end
        // up as an unknown frame [and we don't support frame-specific compression]
        decoded = syncdata::decode(&mut stream);
        stream = BufStream::new(&decoded);
    }

    // Frame grouping. Pretty much nobody uses this, so its ignored.
    if flags & 0x40 != 0 {
        stream.skip(1)?;
    }

    // Encryption is unimplemented, see parse_frame_v3 for more information.
    if flags & 0x4 != 0 {
        warn!("encryption is not supported");
        return Ok(FrameResult::Unknown(
            UnknownFrame::new(frame_id, flags, &stream)
        ));
    }

    // Data length indicator. Some taggers may not flip the data length indicator when
    // compression is enabled, so it's treated as implicitly enabling it.
    // The spec is also vague about whether the length location is affected by the new flag
    // or the existing compression/encryption flags, so we just assume its the latter.
    // Not like it really matters since we always skip this.
    if flags & 0x1 != 0 || flags & 0x8 != 0 {
        stream.skip(4)?;
    }

    // Frame-specific compression.
    if flags & 0x8 != 0 {
        decoded = match inflate_frame(&mut stream) {
            Ok(stream) => stream,
            Err(_) => return Ok(FrameResult::Unknown(
                UnknownFrame::new(frame_id, flags, &stream)
            ))
        };

        stream = BufStream::new(&decoded);
    }

    match_frame_v4(tag_header, frame_id, &mut stream)
}

pub(crate) fn match_frame_v2(
    tag_header: &TagHeader,
    frame_id: &[u8; 3],
    stream: &mut BufStream,
) -> ParseResult<FrameResult> {
    let frame = match frame_id {
        // AttatchedPictureFrame is subtly different in ID3v2.2, so we handle it seperately.
        b"PIC" => frame!(AttachedPictureFrame::parse_v2(stream)?),

        _ => {
            // Convert ID3v2.2 frame IDs to their ID3v2.3 analogues, as this preserves the most frames.
            if let Ok(v3_id) = compat::upgrade_v2_id(frame_id) {
                match_frame_v3(tag_header, v3_id, stream)?
            } else {
                FrameResult::Unknown(UnknownFrame::new(frame_id, 0, stream))
            }
        }
    };

    Ok(frame)
}

pub(crate) fn match_frame_v3(
    tag_header: &TagHeader,
    frame_id: FrameId,
    stream: &mut BufStream,
) -> ParseResult<FrameResult> {
    let frame = match frame_id.inner() {
        // Involved People List
        b"IPLS" => frame!(CreditsFrame::parse(frame_id, stream)?),

        // Relative volume adjustment [Frames 4.12]
        b"RVAD" => frame!(RelativeVolumeFrame::parse(stream)?),

        // Equalisation [Frames 4.13]
        b"EQUA" => frame!(EqualisationFrame::parse(stream)?),

        _ => match_frame(tag_header, frame_id, stream)?,
    };

    Ok(frame)
}

pub(crate) fn match_frame_v4(
    tag_header: &TagHeader,
    frame_id: FrameId,
    stream: &mut BufStream,
) -> ParseResult<FrameResult> {
    // Parse ID3v2.4-specific frames.
    let frame = match frame_id.inner() {
        // Involved People List & Musician Credits List [Frames 4.2.2]
        b"TIPL" | b"TMCL" => frame!(CreditsFrame::parse(frame_id, stream)?),

        // Relative Volume Adjustment 2 [Frames 4.11]
        b"RVA2" => frame!(RelativeVolumeFrame2::parse(stream)?),

        // Equalisation 2 [Frames 4.12]
        b"EQU2" => frame!(EqualisationFrame2::parse(stream)?),

        // Signature Frame [Frames 4.28]
        // b"SIGN" => todo!(),

        // Seek frame [Frames 4.27]
        // b"SEEK" => todo!(),

        // Audio seek point index [Frames 4.30]
        // b"ASPI" => todo!(),
        _ => match_frame(tag_header, frame_id, stream)?,
    };

    Ok(frame)
}

pub(crate) fn match_frame(
    tag_header: &TagHeader,
    frame_id: FrameId,
    stream: &mut BufStream,
) -> ParseResult<FrameResult> {
    // To parse most frames, we have to manually go through and determine what kind of
    // frame to create based on the frame id. There are many frame possibilities, so
    // there are many match arms.

    let frame = match frame_id.inner() {
        // Unique File Identifier [Frames 4.1]
        b"UFID" => frame!(FileIdFrame::parse(stream)?),

        // --- Text Information [Frames 4.2] ---

        // User-Defined Text Information [Frames 4.2.6]
        b"TXXX" => frame!(UserTextFrame::parse(stream)?),

        // Generic Text Information
        _ if TextFrame::is_text(frame_id) => frame!(TextFrame::parse(frame_id, stream)?),

        // --- URL Link [Frames 4.3] ---

        // User-Defined URL Link [Frames 4.3.2]
        b"WXXX" => frame!(UserUrlFrame::parse(stream)?),

        // Generic URL Link
        _ if frame_id.starts_with(b'W') => frame!(UrlFrame::parse(frame_id, stream)?),

        // Music CD Identifier [Frames 4.4]
        // b"MCDI" => todo!(),

        // Event timing codes [Frames 4.5]
        b"ETCO" => frame!(EventTimingCodesFrame::parse(stream)?),

        // MPEG Lookup Codes [Frames 4.6]
        // b"MLLT" => todo!(),

        // Synchronised tempo codes [Frames 4.7]
        // b"SYTC" => todo!(),

        // Unsynchronized Lyrics [Frames 4.8]
        b"USLT" => frame!(UnsyncLyricsFrame::parse(stream)?),

        // Unsynchronized Lyrics [Frames 4.9]
        b"SYLT" => frame!(SyncedLyricsFrame::parse(stream)?),

        // Comments [Frames 4.10]
        b"COMM" => frame!(CommentsFrame::parse(stream)?),

        // (Frames 4.11 & 4.12 are Verson-Specific)

        // Reverb [Frames 4.13]
        // b"RVRB" => todo!(),

        // Attatched Picture [Frames 4.14]
        b"APIC" => frame!(AttachedPictureFrame::parse(stream)?),

        // General Encapsulated Object [Frames 4.15]
        b"GEOB" => frame!(GeneralObjectFrame::parse(stream)?),

        // Play Counter [Frames 4.16]
        b"PCNT" => frame!(PlayCounterFrame::parse(stream)?),

        // Popularimeter [Frames 4.17]
        b"POPM" => frame!(PopularimeterFrame::parse(stream)?),

        // Relative buffer size [Frames 4.18]
        // b"RBUF" => todo!(),

        // Audio Encryption [Frames 4.19]
        // b"AENC" => todo!(),

        // Linked Information [Frames 4.20]
        // b"LINK" => todo!(),

        // Position synchronisation frame [Frames 4.21]
        // b"POSS" => todo!(),

        // Terms of use frame [Frames 4.22]
        b"USER" => frame!(TermsOfUseFrame::parse(stream)?),

        // Ownership frame [Frames 4.23]
        b"OWNE" => frame!(OwnershipFrame::parse(stream)?),

        // Commercial frame [Frames 4.24]
        b"COMR" => frame!(CommercialFrame::parse(stream)?),

        // Encryption Registration [Frames 4.25]
        // b"ENCR" => todo!(),

        // Group Identification [Frames 4.26]
        // b"GRID" => todo!(),

        // Private Frame [Frames 4.27]
        b"PRIV" => frame!(PrivateFrame::parse(stream)?),

        // (Frames 4.28 -> 4.30 are version-specific)

        // Chapter Frame [ID3v2 Chapter Frame Addendum 3.1]
        b"CHAP" => frame!(ChapterFrame::parse(tag_header, stream)?),

        // Table of Contents Frame [ID3v2 Chapter Frame Addendum 3.2]
        b"CTOC" => frame!(TableOfContentsFrame::parse(tag_header, stream)?),

        // iTunes Podcast Frame
        b"PCST" => frame!(PodcastFrame::parse(stream)?),

        // No idea, return unknown frame
        _ => FrameResult::Unknown(UnknownFrame::new(frame_id, 0, stream))
    };

    Ok(frame)
}

#[cfg(feature = "id3v2_zlib")]
fn inflate_frame(src: &mut BufStream) -> ParseResult<Vec<u8>> {
    miniz_oxide::inflate::decompress_to_vec_zlib(src.take_rest()).map_err(|err| {
        warn!("decompression failed: {:?}", err);
        ParseError::MalformedData
    })
}

#[cfg(not(feature = "id3v2_zlib"))]
fn inflate_frame(src: &mut BufStream) -> ParseResult<Vec<u8>> {
    warn!("decompression is not enabled", frame_id);
    Err(ParseError::Unsupported)
}

pub(crate) fn render(tag_header: &TagHeader, frame: &dyn Frame) -> SaveResult<Vec<u8>> {
    // We need to render the frame backwards, starting from the frame and then making the
    // header from the size of that data.

    // Render the frame here, as we will need its size.
    let mut frame_data = frame.render(tag_header);

    if tag_header.version() == Version::V24 && tag_header.flags().unsync {
        // ID3v2.4 global unsync is enabled. Encode our frame.
        frame_data = syncdata::encode(&frame_data);
    }

    let mut data: Vec<u8> = Vec::new();

    data.extend(match tag_header.version() {
        Version::V24 => render_v4_header(frame.id(), frame_data.len())?,
        Version::V23 => render_v3_header(frame.id(), frame_data.len())?,
        Version::V22 => {
            warn!("cannot render ID3v2.2 frames [this is a bug]");
            return Ok(data);
        }
    });

    data.extend(frame_data);

    Ok(data)
}

fn render_v3_header(frame_id: FrameId, size: usize) -> SaveResult<[u8; 10]> {
    let mut data = [0; 10];

    data[0..3].copy_from_slice(frame_id.inner());

    // ID3v2.3 frame sizes are just plain big-endian 32-bit integers, try to fit the value
    // into a u32 and blit it.
    let size: u32 = match size.try_into() {
        Ok(size) => size,
        Err(_) => {
            error!("frame size exceeds the ID3v2.3 limit of 2^32 bytes");
            return Err(SaveError::TooLarge);
        }
    };

    data[4..7].copy_from_slice(&size.to_be_bytes());

    // Leave the flags zeroed. We don't care about them and likely never will.

    Ok(data)
}

fn render_v4_header(frame_id: FrameId, size: usize) -> SaveResult<[u8; 10]> {
    let mut data = [0; 10];

    // First blit the 4-byte ID
    data[0..4].copy_from_slice(frame_id.inner());

    // ID3v2.4 sizes are syncsafe, so the actual limit for them is smaller.
    if size > 256_000_000 {
        error!("frame size exceeds the ID3v2.4 limit of 256mb");
        return Err(SaveError::TooLarge);
    }

    data[4..8].copy_from_slice(&syncdata::from_u28(size as u32));

    // Leave the flags zeroed. We don't care about them and likely never will.

    Ok(data)
}

#[cfg(test)]
mod tests {
    use crate::id3v2::frames::file::PictureType;
    use crate::id3v2::frames::AttachedPictureFrame;
    use crate::id3v2::Tag;
    use std::env;
    
    // TODO: Add tests to make sure that unknown frames are handled right.

    #[macro_export]
    macro_rules! make_frame {
        ($dty:ty, $data:expr, $dest:ident) => {
            crate::make_frame!($dty, $data, crate::id3v2::tag::Version::V24, $dest)
        };

        ($dty:ty, $data:expr, $ver:expr, $dest:ident) => {
            let parsed = crate::id3v2::frames::parse(
                &TagHeader::with_version($ver),
                &mut BufStream::new($data),
            )
            .unwrap();

            let frame = if let crate::id3v2::frames::FrameResult::Frame(frame) = parsed {
                frame
            } else {
                panic!("cannot parse frame: {:?}", parsed)
            };

            let $dest = frame.downcast::<$dty>().unwrap();
        };
    }

    #[macro_export]
    macro_rules! assert_render {
        ($frame:expr, $data:expr) => {
            assert!(!$frame.is_empty());
            assert_eq!(
                crate::id3v2::frames::render(
                    &TagHeader::with_version(crate::id3v2::tag::Version::V24),
                    &$frame
                )
                .unwrap(),
                $data
            )
        };
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
        assert_eq!(apic.picture.len(), 86414);
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
}
