//! Frame parsing and implementations.
//!
//! An ID3v2 tag is primarily made up of chunks of data, called "Frames" by the spec.
//! Frames are highly structured and can contain a variety of information about the audio,
//! including text and binary data.
//!
//! # Using frames
//!
//! TODO
//!
//! # Creating your own frames
//! 
//! TODO
//!
//! More information can be found in the [`Frame`](Frame) definition.

pub mod audio;
pub mod bin;
pub mod chapters;
pub mod comments;
mod encoding;
pub mod events;
pub mod file;
pub mod lyrics;
pub mod owner;
pub mod stats;
pub mod text;
mod types;
pub mod url;

pub use audio::v23::{EqualizationFrame, RelativeVolumeFrame};
pub use audio::v24::{EqualizationFrame2, RelativeVolumeFrame2};
pub use bin::{FileIdFrame, MusicCdIdFrame, PodcastFrame, PrivateFrame};
pub use chapters::{ChapterFrame, TableOfContentsFrame};
pub use comments::CommentsFrame;
pub use events::EventTimingCodesFrame;
pub use file::{AttachedPictureFrame, GeneralObjectFrame};
pub use lyrics::{SyncedLyricsFrame, UnsyncLyricsFrame};
pub use owner::{CommercialFrame, OwnershipFrame, TermsOfUseFrame};
pub use stats::{PlayCounterFrame, PopularimeterFrame};
pub use text::{CreditsFrame, TextFrame, UserTextFrame};
pub use types::*;
pub use url::{UrlFrame, UserUrlFrame};

use crate::core::io::BufStream;
use crate::id3v2::tag::{TagHeader, Version};
use crate::id3v2::{compat, syncdata, ParseError, ParseResult, SaveError, SaveResult};

use dyn_clone::DynClone;
use log::{error, info, warn};
use std::any::Any;
use std::convert::TryInto;
use std::fmt::{Debug, Display};
use std::str;

/// Describes the behavior of a frame implementation.
///
/// Frames are tied to their "Frame ID", which is a unique 4-byte code that identifies how
/// the frame should be parsed. Certain types of Frame IDs are reserved for specific cases.
/// For example, Frame IDs beginning with `T` are reserved for text frames, while Frame IDs
/// beginning with `W` are reserved for URL frames.
///
/// In musikr, all ID3v2 frames are represented using a trait object. This is because frame 
/// information is highly heterogeneous, making other approaches such as enums or a large
/// struct either impractical or prone to error. This trait supplies methods that make working
/// with such an approach much easier.
///
/// **Note:** For simplicity, the flags in the frame header cannot be customized. They will
/// always be written as zeroes.
pub trait Frame: Display + Debug + AsAny + DynClone {
    /// Returns the [`FrameId`](FrameId) of this frame.
    ///
    /// This should not collide with any other frame implementation.
    fn id(&self) -> FrameId;

    /// Returns the unique key of this frame. 
    ///
    /// This is usually the Frame ID followed by whatever information that makes 
    /// this frame unique. For example, the ID3v2 specification states that `APIC`
    /// is made unique by it's Frame ID and it's description, so it's key is 
    /// `APIC:description`. However, `TIPL` is specified as only being unique by
    /// it's Frame ID, so it's key is `TIPL`.
    fn key(&self) -> String;

    /// Returns whether this frame is considered "empty".
    ///
    /// A frame is considered empty whenever there is too little information to
    /// create a valid frame. If this function returns `true`, then the frame will
    /// not be written to the tag if it is saved again.
    fn is_empty(&self) -> bool;

    /// Returns the binary representation of this frame.
    ///
    /// The binary data of the frame should be dynamically generated when this
    /// function is called. The tag header is supplied in the case that a frame
    /// must recurse into sub-frames to fully render the frame.
    ///
    /// **Note:** Do not unsynchronize, compress, or similarly manipulate your
    /// frame data. That will result in a malformed tag.
    fn render(&self, tag_header: &TagHeader) -> Vec<u8>;
}

impl dyn Frame {
    /// Returns whether this frame is an instance of `T`.
    pub fn is<T: Frame>(&self) -> bool {
        self.as_any(Sealed(())).is::<T>()
    }

    /// Fallibly downcasts this frame into a reference to `T`.
    ///
    /// If the frame is not an instance of `T`, `None` is returned.
    pub fn downcast<T: Frame>(&self) -> Option<&T> {
        self.as_any(Sealed(())).downcast_ref::<T>()
    }

    /// Fallibly downcasts this frame into a mutable reference to `T`.
    ///
    /// If the frame is not an instance of `T`, `None` is returned.
    pub fn downcast_mut<T: Frame>(&mut self) -> Option<&mut T> {
        self.as_any_mut(Sealed(())).downcast_mut::<T>()
    }
}

/// This trait is for internal use only.
pub trait AsAny: Any {
    fn as_any(&self, _: Sealed) -> &dyn Any;
    fn as_any_mut(&mut self, _: Sealed) -> &mut dyn Any;
}

impl <T: Frame> AsAny for T {
    fn as_any(&self, _: Sealed) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self, _: Sealed) -> &mut dyn Any {
        self
    }
}

/// Downcasts a boxed `dyn Frame` into a boxed `T`.
///
/// The primary use for this function is the ability to move values in from a
/// concrete downcasted frame instead of cloning. This cannot be included in
/// the `dyn Frame` implementation as such a downcast requires an owned value
/// instead of a reference.
pub fn downcast_into<T: Frame>(frame: Box<dyn Frame>) -> Result<Box<T>, Box<dyn Frame>> {
    if frame.is::<T>() {
        // SAFETY: Checked if this type pointed to T, and since T also implements Frame, we can
        // rely on that check for memory safety because no other impl could conflict with T.
        unsafe { Ok(Box::from_raw(Box::into_raw(frame) as *mut T)) }
    } else {
        Err(frame)
    }
}

dyn_clone::clone_trait_object!(Frame);

/// This type is for internal use only.
pub struct Sealed(());

/// A frame that could not be fully parsed.
///
/// Musikr cannot parse certain frames, such as encrypted frames or ID3v2.2 frames
/// that have no ID3v2.3 analogue. If this is the case, then this struct is returned.
/// `UnknownFrame` instances are immutable and are dropped when a tag is upgraded.
///
/// An UnknownFrame is **not** a [`Frame`](Frame). They can violate certain invariants and cannot be added
/// to a [`FrameMap`](crate::id3v2::collections::FrameMap).
///
/// Generally, these invariants are guaranteed:
/// - The Frame ID is proper ASCII characters or numbers
/// - The frame body has been decoded from the unsynchronization scheme
///
/// These invariants cannot be guaranteed:
/// - The frame ID is 4 bytes
/// - The frame has been fully decompressed
/// - The frame will be sane, even if fully decoded
///
/// Its largely up to the end user to turn an `UnknownFrame` into something usable.
#[derive(Clone, Debug)]
pub struct UnknownFrame {
    frame_id: Vec<u8>,
    flags: u16,
    data: Vec<u8>,
}

impl UnknownFrame {
    fn new<S: AsRef<[u8]>>(frame_id: S, flags: u16, stream: &BufStream) -> Self {
        UnknownFrame {
            frame_id: frame_id.as_ref().to_vec(),
            flags,
            data: stream.to_vec(),
        }
    }

    /// Returns the ID of this tag.
    /// This will be a valid frame ID, but may be 3 bytes or 4 bytes 
    /// depending on the tag version.
    pub fn id(&self) -> &[u8] {
        &self.frame_id
    }

    /// Returns the two flag bytes of this frame.
    /// 
    /// This can be used as a guide for further parsing of this frame.
    pub fn flags(&self) -> u16 {
        self.flags
    }

    /// The data of the frame.
    /// 
    /// This will include the entire frame body, including data length
    /// indicators and other auxiliary data.
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
// There's a reason why we don't include the frame header with frame instances.
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
    // Frame structure differs quite significantly across versions, so we have to
    // handle them separately.
    // TODO: Perhaps make the frame matching and parsing steps fallible.

    match tag_header.version() {
        Version::V22 => parse_frame_v2(tag_header, stream),
        Version::V23 => parse_frame_v3(tag_header, stream),
        Version::V24 => parse_frame_v4(tag_header, stream),
    }
}

fn parse_frame_v2(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<FrameResult> {
    // ID3v2.2 frames are a 3-byte identifier and a 3-byte big-endian size.
    let frame_id = stream.read_array::<3>()?;

    if !FrameId::validate(&frame_id) {
        return Err(ParseError::MalformedData);
    }

    // Make u32::from_be_bytes handle the weird 3-byte sizes
    let mut size_bytes = [0; 4];
    stream.read_exact(&mut size_bytes[1..4])?;
    let size = u32::from_be_bytes(size_bytes) as usize;

    // Luckily for us, we don't need to do any decoding magic for ID3v2.2 frames.
    let mut stream = stream.slice_stream(size)?;

    match_frame_v2(tag_header, &frame_id, &mut stream)
}

fn parse_frame_v3(tag_header: &TagHeader, stream: &mut BufStream) -> ParseResult<FrameResult> {
    let id_bytes = stream.read_array()?;
    let size = stream.read_u32()? as usize;
    let flags = stream.read_u16()?;

    // Technically, the spec says that empty frames should be a sign of a malformed tag, but they're
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
    let frame_id = match FrameId::try_new(&id_bytes) {
        Ok(id) => id,
        Err(_) => {
            if FrameId::validate(&id_bytes[0..3]) && id_bytes[3] == 0 {
                info!("correcting incorrect ID3v2.2 frame ID");

                let mut v2_id = [0; 3];
                v2_id.copy_from_slice(&id_bytes[0..3]);

                // Unsure if taggers will write full ID3v2.2 frames or just the IDs. Assume
                // its the latter.
                return match_frame_v2(tag_header, &v2_id, &mut stream);
            }

            return Err(ParseError::MalformedData);
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
        return Ok(FrameResult::Unknown(UnknownFrame::new(
            frame_id, flags, &stream,
        )));
    }

    // Frame-specific compression. This flag also adds a data length indicator that we will skip.
    if flags & 0x80 != 0 {
        stream.skip(4)?;

        decoded = match inflate_frame(&mut stream) {
            Ok(stream) => stream,
            Err(_) => {
                return Ok(FrameResult::Unknown(UnknownFrame::new(
                    frame_id, flags, &stream,
                )))
            }
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
    let frame_id = match FrameId::try_new(&stream.read_array()?) {
        Ok(id) => id,
        Err(_) => return Err(ParseError::MalformedData),
    };

    // ID3v2.4 sizes *should* be syncsafe, but iTunes wrote v2.3-style sizes for awhile. Fix that.
    let size_bytes = stream.read_array()?;
    let mut size = syncdata::to_u28(size_bytes) as usize;

    if size >= 0x80 {
        let mut next_id = [0; 4];

        if let Ok(id) = stream.peek(size + 2..size + 6) {
            next_id.copy_from_slice(id)
        }

        if next_id[0] != 0 && !FrameId::validate(&next_id) {
            // If the raw size leads us to the next frame where the "syncsafe"
            // size wouldn't, we will use that size instead.
            let v3_size = u32::from_be_bytes(size_bytes) as usize;

            if let Ok(id) = stream.peek(v3_size + 2..v3_size + 6) {
                next_id.copy_from_slice(id)
            }

            if FrameId::validate(&next_id) {
                info!("correcting non-syncsafe ID3v2.4 frame size");
                size = v3_size;
            }
        }
    }

    let flags = stream.read_u16()?;

    // Technically, the spec says that empty frames should be a sign of a malformed tag, but they're
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

    // Frame-specific unsynchronization. The spec is vague about whether the non-size bytes
    // are affected by unsynchronization, so we just assume that they are.
    if flags & 0x2 != 0 || tag_header.flags().unsync {
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
        return Ok(FrameResult::Unknown(UnknownFrame::new(
            frame_id, flags, &stream,
        )));
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
            Err(_) => {
                return Ok(FrameResult::Unknown(UnknownFrame::new(
                    frame_id, flags, &stream,
                )))
            }
        };

        stream = BufStream::new(&decoded);
    }

    match_frame_v4(tag_header, frame_id, &mut stream)
}

// --------
// To parse most frames, we have to manually go through and determine what kind of
// frame to create based on the frame id. There are many frame possibilities, so
// there are many match arms.
// Note that some frame specs are commented out. This is intentional, as I there are
// some frame specifications that are so obscure as to not really need to be implemented.
// --------

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
            match compat::upgrade_v2_id(frame_id) {
                Ok(v3_id) => match_frame_v3(tag_header, v3_id, stream)?,
                Err(_) => FrameResult::Unknown(UnknownFrame::new(frame_id, 0, stream)),
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
    let frame = match frame_id.as_ref() {
        // Involved People List
        b"IPLS" => frame!(CreditsFrame::parse(frame_id, stream)?),

        // Relative volume adjustment [Frames 4.12]
        b"RVAD" => frame!(RelativeVolumeFrame::parse(stream)?),

        // Equalization [Frames 4.13]
        b"EQUA" => frame!(EqualizationFrame::parse(stream)?),

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
    let frame = match frame_id.as_ref() {
        // Involved People List & Musician Credits List [Frames 4.2.2]
        b"TIPL" | b"TMCL" => frame!(CreditsFrame::parse(frame_id, stream)?),

        // Relative Volume Adjustment 2 [Frames 4.11]
        b"RVA2" => frame!(RelativeVolumeFrame2::parse(stream)?),

        // Equalization 2 [Frames 4.12]
        b"EQU2" => frame!(EqualizationFrame2::parse(stream)?),

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
    let frame = match frame_id.as_ref() {
        // Unique File Identifier [Frames 4.1]
        b"UFID" => frame!(FileIdFrame::parse(stream)?),

        // --- Text Information [Frames 4.2] ---

        // Generic Text Information
        _ if TextFrame::is_id(frame_id) => frame!(TextFrame::parse(frame_id, stream)?),

        // User-Defined Text Information [Frames 4.2.6]
        b"TXXX" => frame!(UserTextFrame::parse(stream)?),

        // --- URL Link [Frames 4.3] ---

        // Generic URL Link
        _ if UrlFrame::is_id(frame_id) => frame!(UrlFrame::parse(frame_id, stream)?),

        // User-Defined URL Link [Frames 4.3.2]
        b"WXXX" => frame!(UserUrlFrame::parse(stream)?),

        // Music CD Identifier [Frames 4.4]
        b"MCDI" => frame!(MusicCdIdFrame::parse(stream)?),

        // Event timing codes [Frames 4.5]
        b"ETCO" => frame!(EventTimingCodesFrame::parse(stream)?),

        // MPEG Lookup Codes [Frames 4.6]
        // b"MLLT" => todo!(),

        // Synchronized tempo codes [Frames 4.7]
        // b"SYTC" => todo!(),

        // Unsynchronized Lyrics [Frames 4.8]
        b"USLT" => frame!(UnsyncLyricsFrame::parse(stream)?),

        // Unsynchronized Lyrics [Frames 4.9]
        b"SYLT" => frame!(SyncedLyricsFrame::parse(stream)?),

        // Comments [Frames 4.10]
        b"COMM" => frame!(CommentsFrame::parse(stream)?),

        // (Frames 4.11 & 4.12 are Version-Specific)

        // Reverb [Frames 4.13]
        // b"RVRB" => todo!(),

        // Attached Picture [Frames 4.14]
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

        // Position synchronization frame [Frames 4.21]
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

        // No idea, return an unknown frame
        _ => FrameResult::Unknown(UnknownFrame::new(frame_id, 0, stream)),
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
    warn!("decompression is not enabled");
    Err(ParseError::Unsupported)
}

pub(crate) fn render(tag_header: &TagHeader, frame: &dyn Frame) -> SaveResult<Vec<u8>> {
    assert_ne!(tag_header.version(), Version::V22);

    // We need to render the frame backwards, starting from the frame and then making the
    // header from the size of that data.

    // Render the frame here, as we will need its size.
    let frame_data = frame.render(tag_header);
    let mut data: Vec<u8> = Vec::new();

    // Render the header. Leave the flags zeroed, we don't care about them and likely
    // never will.
    data.extend(match tag_header.version() {
        Version::V24 => render_v4_header(frame.id(), 0, frame_data.len())?,
        Version::V23 => render_v3_header(frame.id(), 0, frame_data.len())?,
        Version::V22 => unreachable!(),
    });

    data.extend(frame_data);

    Ok(data)
}

pub(crate) fn render_unknown(tag_header: &TagHeader, frame: &UnknownFrame) -> Vec<u8> {
    assert_ne!(tag_header.version(), Version::V22);

    // Unknown frames with ID3v2.2 IDs can't be rendered.
    if frame.id().len() < 4 {
        warn!("dropping unwritable unknown frame {}", frame.id_str());
        return Vec::new();
    }

    let frame_id = FrameId::new(&frame.id().try_into().unwrap());

    let mut data: Vec<u8> = Vec::new();

    // UnknownFrame instances are immutable, so we can assume that they will render with no issues.
    // We also re-render the unknown frame flags as well, exlcuding the ID3v2.4 unsync flag, as we don't
    // resynchronize frames.
    data.extend(match tag_header.version() {
        Version::V24 => {
            render_v4_header(frame_id, frame.flags() & 0xFFFD, frame.data().len()).unwrap()
        }
        Version::V23 => render_v3_header(frame_id, frame.flags(), frame.data().len()).unwrap(),
        Version::V22 => unreachable!(),
    });

    data.extend(frame.data());

    data
}

fn render_v3_header(frame_id: FrameId, flags: u16, size: usize) -> SaveResult<[u8; 10]> {
    let mut data = [0; 10];

    data[0..4].copy_from_slice(frame_id.as_ref());

    // ID3v2.3 frame sizes are just plain big-endian 32-bit integers, try to fit the value
    // into a u32 and blit it.
    let size: u32 = match size.try_into() {
        Ok(size) => size,
        Err(_) => {
            error!("frame size exceeds the ID3v2.3 limit of 2^32 bytes");
            return Err(SaveError::TooLarge);
        }
    };

    data[4..8].copy_from_slice(&size.to_be_bytes());

    // Render flags.
    data[8] = (flags & 0xFF00) as u8;
    data[9] = (flags & 0x00FF) as u8;

    Ok(data)
}

fn render_v4_header(frame_id: FrameId, flags: u16, size: usize) -> SaveResult<[u8; 10]> {
    let mut data = [0; 10];

    // First blit the 4-byte ID
    data[0..4].copy_from_slice(frame_id.as_ref());

    // ID3v2.4 sizes are syncsafe, so the actual limit for them is smaller.
    if size > 256_000_000 {
        error!("frame size exceeds the ID3v2.4 limit of 256mb");
        return Err(SaveError::TooLarge);
    }

    data[4..8].copy_from_slice(&syncdata::from_u28(size as u32));

    // Render flags.
    data[8] = (flags & 0xFF00) as u8;
    data[9] = (flags & 0x00FF) as u8;

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::frames::file::PictureType;
    use crate::id3v2::Tag;
    use std::env;
    use std::ops::Deref;

    const DATA_V2: &[u8] = b"TT2\x00\x00\x09\x00Unspoken";
    const DATA_V3: &[u8] = b"TIT2\x00\x00\x00\x09\x00\x00\x00Unspoken";
    const DATA_V4: &[u8] = b"TIT2\x00\x00\x00\x09\x00\x00\x00Unspoken";

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

    #[test]
    fn handle_frame_v2() {
        let frame = parse(
            &TagHeader::with_version(Version::V22),
            &mut BufStream::new(DATA_V2),
        )
        .unwrap();

        if let FrameResult::Frame(frame) = frame {
            assert_eq!(frame.id(), b"TIT2");
            assert_eq!(
                render(&TagHeader::with_version(Version::V23), frame.deref()).unwrap(),
                DATA_V3
            );
        } else {
            panic!("frame was not parsed");
        }
    }

    #[test]
    fn handle_frame_v3() {
        let frame = parse(
            &TagHeader::with_version(Version::V23),
            &mut BufStream::new(DATA_V3),
        )
        .unwrap();

        if let FrameResult::Frame(frame) = frame {
            assert_eq!(frame.id(), b"TIT2");
            assert_eq!(
                render(&TagHeader::with_version(Version::V24), frame.deref()).unwrap(),
                DATA_V4
            );
        } else {
            panic!("frame was not parsed");
        }
    }

    #[test]
    fn handle_frame_v4() {
        let frame = parse(
            &TagHeader::with_version(Version::V24),
            &mut BufStream::new(DATA_V4),
        )
        .unwrap();

        if let FrameResult::Frame(frame) = frame {
            assert_eq!(frame.id(), b"TIT2");
            assert_eq!(
                render(&TagHeader::with_version(Version::V23), frame.deref()).unwrap(),
                DATA_V3
            );
        } else {
            panic!("frame was not parsed");
        }
    }

    #[test]
    fn handle_unknown_v2() {
        let data = b"ABC\x00\x00\x04\x16\x16\x16\x16";

        let frame = parse(
            &TagHeader::with_version(Version::V22),
            &mut BufStream::new(data),
        )
        .unwrap();

        if let FrameResult::Unknown(unknown) = frame {
            assert_eq!(unknown.id(), b"ABC");
            assert_eq!(unknown.flags(), 0);
            assert_eq!(unknown.data(), b"\x16\x16\x16\x16");

            assert!(render_unknown(&TagHeader::with_version(Version::V23), &unknown).is_empty());
        } else {
            panic!("frame is not unknown")
        }
    }

    #[test]
    fn handle_unknown_v3() {
        let data = b"APIC\x00\x00\x00\x09\x00\x60\
                     \x16\x12\x34\x56\x78\x9A\xBC\xDE\xF0";

        let frame = parse(
            &TagHeader::with_version(Version::V23),
            &mut BufStream::new(data),
        )
        .unwrap();

        if let FrameResult::Unknown(unknown) = frame {
            assert_eq!(unknown.id(), b"APIC");
            assert_eq!(unknown.flags(), 0x0060);
            assert_eq!(unknown.data(), b"\x16\x12\x34\x56\x78\x9A\xBC\xDE\xF0");

            assert_eq!(
                render_unknown(&TagHeader::with_version(Version::V23), &unknown),
                data
            );
        } else {
            panic!("frame is not unknown")
        }
    }

    #[test]
    fn handle_unknown_v4() {
        let data = b"TIT2\x00\x00\x00\x0F\x00\x06\
                     \x16\x16\x16\x16\xFF\x00\xE0\x12\x34\x56\x78\x9A\xBC\xDE\xF0";

        let out = b"TIT2\x00\x00\x00\x0E\x00\x04\
                    \x16\x16\x16\x16\xFF\xE0\x12\x34\x56\x78\x9A\xBC\xDE\xF0";

        let frame = parse(
            &TagHeader::with_version(Version::V24),
            &mut BufStream::new(data),
        )
        .unwrap();

        if let FrameResult::Unknown(unknown) = frame {
            assert_eq!(unknown.id(), b"TIT2");
            assert_eq!(unknown.flags(), 0x006);
            assert_eq!(
                unknown.data(),
                b"\x16\x16\x16\x16\xFF\xE0\x12\x34\x56\x78\x9A\xBC\xDE\xF0"
            );

            assert_eq!(
                render_unknown(&TagHeader::with_version(Version::V24), &unknown),
                out
            );
        } else {
            panic!("frame is not unknown")
        }
    }
}
