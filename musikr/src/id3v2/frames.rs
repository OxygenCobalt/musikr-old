//! Frame parsing and implementations.
//!
//! An ID3v2 tag is primarily made up of chunks of data, called "Frames" by the spec.
//! Frames are highly structured and can contain a variety of information about the media,
//! including text, URLs, binary data, and audio adjustments.
//!
//! # Working with `dyn Frame`
//!
//! The ID3v2 module is somewhat unorthodox in that it chooses to represent frames as a
//! trait object of [`Frame`](Frame) instead of an enum. This is for the following reasons:
//!
//! - Trait objects make it easy to implement new frame specifications, even outside of musikr.
//! - There are numerous frames, each with their own distinct structure. This would make a large
//! enum extremely cumbersome to use.
//!
//! Musikr attempts to alleviate the shortcomings of trait objects by implementing
//! methods to downcast frames to a concrete type. For example, a `dyn Frame` instance
//! could be transformed into a (mutable) reference to a concrete type with
//! [`Frame::downcast`]((Frame)::downcast) and [`Frame::downcast_mut`]((Frame)::downcast_mut),
//! respectively.
//!
//! ```rust
//! use musikr::{text_frame, id3v2::frames::{Frame, TextFrame}};
//!
//! let mut frame: Box<dyn Frame> = Box::new(text_frame! {
//!     b"TIT2", ["Title"]
//! });
//!
//! // We know that this is a text frame, so we can unwrap. You
//! // won't want to do this if you're unsure of the frame type.
//! let downcasted = frame.downcast_mut::<TextFrame>().unwrap();
//! downcasted.text[0] = String::from("Another Title");
//!
//! let downcasted: &TextFrame = frame.downcast::<TextFrame>().unwrap();
//! assert_eq!(&downcasted.text[0], "Another Title");
//! ```
//!
//! To get an *owned* concrete type  back, [`frames::downcast_box`](downcast_box) can be used.
//! This function will only work on boxed `dyn Frame` instances, and is not bound to the `Frame`
//! implementation due to limitations  with external types.
//!
//! It is highly recommended to use this function over [`Box::downcast`](Box::downcast). The latter
//! method will upcast to `Any`, which may result in the downcast failing as the type specified will
//! not match.
//!
//! As for cloning frames, [`DynClone`](dyn_clone) is automatically implemented for the trait, so
//! cloning is possible with [`dyn_clone::clone`](dyn_clone::clone) and  [`dyn_clone::clone_box`](dyn_clone::clone_box).
//!
//! # Creating a custom frame
//!
//! A custom frame can be created by implementing [`Frame`](Frame) on the desired datatype. However,
//! this only enables a frame to be written to a file. To allow a custom frame to be read from a file,
//! a custom [`FrameParser`](FrameParser) implementation must be created and provided to a [`Tag`](crate::id3v2::Tag)
//! with [`Tag::open_with_parser`](crate::id3v2::Tag::open_with_parser). More information can be found
//! in the linked documentation.
//!
//! ## Example
//!
//! This custom `Frame` and `FrameParser` implements a custom play counter frame, similar to `PCNT`.
//!
//! ```rust
//! use musikr::id3v2::{
//!     Tag, ParseResult, tag::TagHeader,
//!     frames::{Frame, FrameId, FrameParser, FrameData, FrameResult, DefaultFrameParser}
//! };
//! use musikr::core::BufStream;
//! use std::fmt::{self, Display, Formatter};
//!
//! #[derive(Debug, Clone)]
//! pub struct MyPlayCountFrame {
//!     pub count: u64
//! }
//!
//! impl MyPlayCountFrame {
//!     fn parse(mut stream: BufStream) -> ParseResult<Self> {
//!         Ok(Self { count: stream.read_be_u64()? })
//!     }
//! }
//!
//! impl Frame for MyPlayCountFrame {
//!     fn id(&self) -> FrameId {
//!         FrameId::new(b"XCNT")
//!     }
//!
//!     fn key(&self) -> String {
//!         String::from("XCNT")
//!     }
//!
//!     fn is_empty(&self) -> bool {
//!         // Whats the point of writing that a file has zero plays?
//!         self.count == 0
//!     }
//!
//!     fn render(&self, _: &TagHeader) -> Vec<u8> {
//!         Vec::from(self.count.to_be_bytes())
//!     }
//! }
//!
//! impl Display for MyPlayCountFrame {
//!     fn fmt(&self, f: &mut Formatter) -> fmt::Result {
//!         write![f, "plays: {}", self.count]
//!     }
//! }
//!
//! pub struct MyFrameParser;
//!
//! impl FrameParser for MyFrameParser {
//!     fn parse<'a>(&self, tag_header: &TagHeader, data: FrameData<'a>) -> ParseResult<FrameResult<'a>> {
//!         // We want to eliminate the possibility that we accidentally parse a frame that is already
//!         // represented by something in musikr, so first run this data through DefaultFrameParser.
//!         let sup = DefaultFrameParser::default();
//!
//!         match sup.parse(tag_header, data) {
//!             Ok(FrameResult::Frame(frame)) => Ok(FrameResult::Frame(frame)),
//!             Ok(FrameResult::Unknown(data)) => {
//!                 // The default parser did not recognize this frame, we can try to parse it now.
//!                 let result = match data {
//!                     // Lets just say that the frame can exist on ID3v2.2, ID3v2.3, and ID3v2.4.
//!                     FrameData::Legacy(frame_id, stream) if &frame_id == b"XCT" => FrameResult::Frame(Box::new(MyPlayCountFrame::parse(stream)?)),
//!                     FrameData::Normal(frame_id, stream) if frame_id == b"XCNT" => FrameResult::Frame(Box::new(MyPlayCountFrame::parse(stream)?)),
//!
//!                     // We couldn't discern the frame either.
//!                     _ => FrameResult::Unknown(data)
//!                 };
//!
//!                 Ok(result)
//!             },
//!             // The default parser errored or dropped the frame. Propagate the error.
//!             fail => fail
//!         }
//!     }
//! }
//!
//! # use std::error::Error;
//! # use std::env;
//! # fn main() -> Result<(), Box<dyn Error>> {
//! #   let example_path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/example.mp3";
//! #   let out_path = env::temp_dir().join("musikr_frame_ex.mp3");
//! let mut tag = Tag::open(&example_path)?;
//! let frame = MyPlayCountFrame { count: 16 };
//! tag.frames.insert(frame);
//! tag.save(&out_path);
//!
//! let tag = Tag::open_with_parser(&out_path, &MyFrameParser)?;
//! assert_eq!(tag.frames["XCNT"].downcast::<MyPlayCountFrame>().unwrap().count, 16);
//! #   Ok(())
//! # }
//! ```

pub mod audio;
pub mod bin;
pub mod chapters;
mod encoding;
pub mod events;
pub mod file;
pub mod lyrics;
pub mod owner;
pub mod stats;
pub mod text;
pub mod url;

pub use audio::v23::{EqualizationFrame, RelativeVolumeFrame};
pub use audio::v24::{EqualizationFrame2, RelativeVolumeFrame2};
pub use bin::{FileIdFrame, MusicCdIdFrame, PodcastFrame, PrivateFrame};
pub use chapters::{ChapterFrame, TableOfContentsFrame};
pub use events::EventTimingCodesFrame;
pub use file::{AttachedPictureFrame, GeneralObjectFrame};
pub use lyrics::{SyncedLyricsFrame, UnsyncLyricsFrame};
pub use owner::{CommercialFrame, OwnershipFrame, TermsOfUseFrame};
pub use stats::{PlayCounterFrame, PopularimeterFrame};
pub use text::{CommentsFrame, CreditsFrame, TextFrame, UserTextFrame};
pub use url::{UrlFrame, UserUrlFrame};

use crate::core::io::BufStream;
use crate::id3v2::tag::{TagHeader, Version};
use crate::id3v2::{compat, syncdata, ParseError, ParseResult, SaveError, SaveResult};

use dyn_clone::DynClone;
use log::{error, info, warn};
use std::any::Any;
use std::fmt::{Debug, Display};
use std::str::{self, FromStr};

/// Describes the behavior of a frame implementation.
///
/// This trait is used as both a specification of frame behavior, and a trait object to
/// represent any frame in a collections. Implementing this trait alone will only enable
/// writing a frame to a tag. To implement reading a custom frame from a tag, see
/// [`FrameParser`](FrameParser).
///
/// # Examples
///
/// An example of a custom `Frame` implementation can be found in the [module documentation](self)
pub trait Frame: Display + Debug + AsAny + DynClone {
    /// Returns the [`FrameId`](FrameId) of this frame.
    ///
    /// # Custom Frame Considerations
    /// - Certain Frame ID namespaces are reserved. For example, all frames beginning with `T` are
    /// reserved for text frames, while all frames beginning with `W` are reserved for URL frames.
    /// Keep this in mind depending on the frame you are intending to create.
    /// - Generally, all Frame IDs beginning with `X`, `Y`, and `Z` are free for anyone to use, while
    /// the remaining Frame IDs are reserved by the specification. This is reccomended, but not required
    /// by musikr.
    /// - Do not use a Frame ID already used by the standard. This may result in errors in musikr or other
    /// programs.
    fn id(&self) -> FrameId;

    /// Returns the unique key of this frame.
    ///
    /// This should be the Frame ID, followed by any information that marks this frame as "unique" from other
    /// frames, delimited by a `:`. A key consisting of only the Frame ID means that there should only be one
    /// frame in the tag,
    ///
    /// # Examples
    ///
    /// ```text
    /// TIT2 -> There should only be one TIT2 frame in this tag.
    /// APIC:description -> There can be multiple APIC frames in a tag, as long as they
    /// have different descriptions.
    /// COMM:description:eng -> There can be multiple COMM frames in a tag, as long as the
    /// descriptions or language differs.
    /// ```
    fn key(&self) -> String;

    /// Returns whether this frame is considered "empty".
    ///
    /// A frame is considered empty whenever there is too little information to
    /// create a valid frame. If this function returns `true`, then the frame will
    /// not be written to the tag if it is saved again.
    ///
    /// # Examples
    ///
    /// ```
    /// use musikr::{text_frame, id3v2::frames::{Frame, TextFrame}};
    /// let empty = text_frame! { b"TIT2", ["", ""] };
    /// let emptier = text_frame! { b"TIT2" };
    ///
    /// // Text frames must have at least one string to write, so an empty list
    /// // or a list of empty strings mean that the frame is empty.
    /// assert!(empty.is_empty());
    /// assert!(emptier.is_empty());
    /// ```
    fn is_empty(&self) -> bool;

    /// Generates the binary representation of this frame.
    ///
    /// When called, the body of the frame should by dynamically generated.
    /// Musikr will then add the frame header and write the frame. If a situation
    /// occurs where a field cannot be transformed, the function should not panic
    /// in favor of the data degrading gracefully.
    ///
    /// **Note:** Do not unsynchronize, compress, or similarly manipulate
    /// your frame data. The frame header's flags are always zeroed, so
    /// doing such will result in an unreadable frame.
    fn render(&self, tag_header: &TagHeader) -> Vec<u8>;
}

impl dyn Frame {
    /// Returns whether this frame is an instance of `T`.
    pub fn is<T: Frame>(&self) -> bool {
        self.__as_any(Sealed(())).is::<T>()
    }

    /// Fallibly downcasts this frame into a reference to `T`.
    ///
    /// If the frame is not an instance of `T`, `None` is returned.
    pub fn downcast<T: Frame>(&self) -> Option<&T> {
        self.__as_any(Sealed(())).downcast_ref::<T>()
    }

    /// Fallibly downcasts this frame into a mutable reference to `T`.
    ///
    /// If the frame is not an instance of `T`, `None` is returned.
    pub fn downcast_mut<T: Frame>(&mut self) -> Option<&mut T> {
        self.__as_any_mut(Sealed(())).downcast_mut::<T>()
    }
}

#[doc(hidden)]
pub struct Sealed(());

#[doc(hidden)]
pub trait AsAny: Any {
    fn __as_any(&self, _: Sealed) -> &dyn Any;
    fn __as_any_mut(&mut self, _: Sealed) -> &mut dyn Any;
}

impl<T: Frame> AsAny for T {
    fn __as_any(&self, _: Sealed) -> &dyn Any {
        self
    }

    fn __as_any_mut(&mut self, _: Sealed) -> &mut dyn Any {
        self
    }
}

dyn_clone::clone_trait_object!(Frame);

/// Downcasts a boxed `dyn Frame` into a boxed `T`.
///
/// The primary use for this function is the ability to move values in from a
/// concrete downcasted frame instead of cloning. This cannot be included in
/// the `dyn Frame` implementation as such a downcast requires an owned value
/// instead of a reference.
///
/// # Errors
///
/// If `frame` is not `T`, then an error will be returned with the original data.
///
/// # Example
///
/// ```
/// use musikr::{text_frame, id3v2::frames::{self, Frame, TextFrame}};
/// let frame: Box<dyn Frame> = Box::new(text_frame! { b"TIT2", ["Title"] });
/// // This is not a shared reference, so we can move the text value out.
/// let text = frames::downcast_box::<TextFrame>(frame).unwrap().text;
/// assert_eq!(text, &["Title"])
/// ```
pub fn downcast_box<T: Frame>(frame: Box<dyn Frame>) -> Result<Box<T>, Box<dyn Frame>> {
    if frame.is::<T>() {
        // SAFETY: Checked if this type pointed to T, and since T also implements Frame, we can
        // rely on that check for memory safety because no other impl could conflict with T.
        // I don't fully know the semantics around pointer casting, but this seems to work
        // so it's probably fine. really hope I'm not transmuting with extra steps here.
        unsafe { Ok(Box::from_raw(Box::into_raw(frame) as *mut T)) }
    } else {
        Err(frame)
    }
}

/// A frame that could not be fully parsed.
///
/// Musikr cannot parse certain frames, such as encrypted frames or ID3v2.2 frames
/// that have no ID3v2.3 analogue. If this is the case, then this struct is returned.
/// `UnknownFrame` instances are immutable and are dropped when a tag is upgraded.
///
/// An UnknownFrame is **not** a [`Frame`](Frame). They can violate certain invariants
/// and cannot be added to a [`FrameMap`](crate::id3v2::collections::FrameMap).
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
    fn new(data: FrameData, flags: u16) -> Self {
        let (frame_id, data) = match data {
            FrameData::Normal(frame_id, stream) => (frame_id.as_ref().to_vec(), stream.to_vec()),
            FrameData::Legacy(frame_id, stream) => (frame_id.to_vec(), stream.to_vec()),
        };

        Self {
            frame_id,
            flags,
            data,
        }
    }

    /// Returns the ID of this tag.
    ///
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


/// A representation of an ID3v2.3 or ID3v2.4 Frame ID.
///
/// Frame IDs are 4-byte sequences consisting of uppercase ASCII characters or
/// numbers.
///
/// # Example
/// ```
/// use musikr::id3v2::frames::FrameId;
///
/// let alpha = FrameId::try_new(b"APIC");
/// let numeric = FrameId::try_new(b"1234");
/// let both = FrameId::try_new(b"TPE3");
/// let bad = FrameId::try_new(b"apic");
///
/// assert!(matches!(alpha, Ok(_)));
/// assert!(matches!(numeric, Ok(_)));
/// assert!(matches!(both, Ok(_)));
/// assert!(matches!(bad, Err(_)));
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FrameId([u8; 4]);

impl FrameId {
    /// Creates an instance.
    ///
    /// # Panics
    /// This function will panic if `id` is not a valid language code.
    /// If the validity of the input cannot be assured,
    /// [`try_new`](FrameId::try_new) should be used instead.
    pub fn new(id: &[u8; 4]) -> Self {
        Self::try_new(id).unwrap()
    }

    /// Fallibly creates an instance.
    ///
    /// # Errors
    /// If `id` is not a valid Frame ID, then an error will be returned.
    pub fn try_new(id: &[u8; 4]) -> Result<Self, FrameIdError> {
        if !Self::validate(id) {
            return Err(FrameIdError(()));
        }

        Ok(Self(*id))
    }

    /// Returns a copy of the internal array of this instance.
    pub fn inner(&self) -> [u8; 4] {
        self.0
    }

    /// Interprets this Frame ID s a string.
    pub fn as_str(&self) -> &str {
        // We've asserted that this frame is ASCII, so we can unwrap.
        str::from_utf8(&self.0).unwrap()
    }

    pub(crate) fn validate(frame_id: &[u8]) -> bool {
        for ch in frame_id {
            // Valid frame IDs can only contain uppercase ASCII chars and numbers.
            if !(b'A'..=b'Z').contains(ch) && !(b'0'..=b'9').contains(ch) {
                return false;
            }
        }

        true
    }
}

impl_array_newtype!(FrameId, FrameIdError, 4);
impl_newtype_err! {
    /// The error returned when a [`FrameId`](FrameId) is not valid.
    FrameIdError => "frame id was not a 4-byte sequence of uppercase ascii characters or digits"
}

impl TryFrom<&[u8]> for FrameId {
    type Error = FrameIdError;

    fn try_from(other: &[u8]) -> Result<Self, Self::Error> {
        match other.try_into() {
            Ok(arr) => Self::try_new(&arr),
            Err(_) => Err(FrameIdError(())),
        }
    }
}

impl FromStr for FrameId {
    type Err = FrameIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 4 {
            return Err(FrameIdError(()));
        }

        let mut id = [0; 4];

        for (i, ch) in s.chars().enumerate() {
            if !('A'..='Z').contains(&ch) && !('0'..='9').contains(&ch) {
                return Err(FrameIdError(()));
            }

            id[i] = ch as u8;
        }

        Ok(FrameId(id))
    }
}

// --------
// This is where things get frustratingly messy. The ID3v2 spec over-engineers frames
// to such an extent where that most of the instantiation and parsing code is a horrific tangle
// of control flow, match statements, sanity checks, and quirk workarounds to get a [mostly]
// working frame. There's a reason why we don't include the frame header with frame instances.
// You have been warned.
// --------

/// Specifies the behavior of a custom frame parser.
///
/// Implementing this on a type allows custom frame implementations to be parsed from a file,
/// as long as it is provided to the parsing process with [`Tag::open_with_parser`](crate::id3v2::Tag::open_with_parser).
///
/// Using a custom parser can be dangerous, as if a Frame ID already represented by musikr is
/// accidentally parsed to a different type, it may result in unexpected behavior in musikr
/// or in external programs. When parsing, it is highly recommended to always call
/// [`DefaultFrameParser`](DefaultFrameParser) before your custom logic, so that any IDs already
/// handled by musikr will be assigned to their specific implementation.
///
/// # Examples
///
/// An example of a custom `FrameParser` used in conjunction with a custom [`Frame`](Frame) implementation
/// can be found in the [module documentation](self).
pub trait FrameParser {
    /// Determine and parse a frame from the given data.
    ///
    /// It's not recommended to parse frame data solely based off [`TagHeader::version`](TagHeader::version).
    /// This is because there are cases where the version will be [Version::V23](Version::V23), but the `data`
    /// will still be [`FrameData::Legacy`](FrameData::Legacy). The `data` field should first be unpacked, and then
    /// logic can be continued from there.
    fn parse<'a>(
        &self,
        tag_header: &TagHeader,
        data: FrameData<'a>,
    ) -> ParseResult<FrameResult<'a>>;
}

/// Frame data that has not been fully parsed.
///
/// The data in this enum has already had the header parsed and the frame body fully decoded,
/// but has not been parsed into a [`Frame`](Frame) implementation yet. This is mot commonly
/// used with [`FrameParser`](FrameParser)
pub enum FrameData<'a> {
    /// Legacy ID3v2.2 frame data, with a 3-byte ID. This can originate
    /// from an ID3v2.2 tag, or from an ID3v2.3 tag that still uses ID3v2.2
    /// tags. It's recommended to translate the frame data into an ID3v2.3
    /// frame, as that ensures maximum compatibility with the standard.
    Legacy([u8; 3], BufStream<'a>),

    /// Normal frame data for ID3v2.3+, This can be parsed normally.
    Normal(FrameId, BufStream<'a>),
}

/// A result returned from the use of a [`FrameParser`] implementation.
pub enum FrameResult<'a> {
    /// A frame implementation was successfully parsed.
    Frame(Box<dyn Frame>),
    /// The frame could not be fully parsed or decoded.
    Unknown(FrameData<'a>),
    /// The frame should be ignored.
    Dropped,
}

/// The default [`FrameParser`](FrameParser) used by the tag parsing process.
///
/// This parser supports the most common parts of the ID3v2 specification, including
/// mapping legacy ID3v2.2 frames to modern ID3v2.3 frames. This implementation will be
/// used by [`Tag::open`](crate::id3v2::Tag::open) when parsing a tag.
#[derive(Debug, Clone, Copy)]
pub struct DefaultFrameParser {
    /// Whether to error out when a frame fails to parse. If this is true,
    /// tag parsing will stop as soon as a frame handled by this implementation
    /// fails to correctly parse.
    pub strict: bool,
}

impl FrameParser for DefaultFrameParser {
    fn parse<'a>(
        &self,
        tag_header: &TagHeader,
        data: FrameData<'a>,
    ) -> ParseResult<FrameResult<'a>> {
        let result = match data {
            FrameData::Legacy(frame_id, stream) => {
                self.match_frame_v2(tag_header, frame_id, stream)
            }

            FrameData::Normal(frame_id, stream) => match tag_header.version() {
                Version::V22 => unreachable!(),
                Version::V23 => self.match_frame_v3(tag_header, frame_id, stream),
                Version::V24 => self.match_frame_v4(tag_header, frame_id, stream),
            },
        };

        match result {
            Ok(frame) => Ok(frame),
            Err(err) => {
                error!("frame could not be parsed: {}", err);

                if self.strict {
                    Err(err)
                } else {
                    warn!("strict mode is not on, so this frame will be dropped.");
                    Ok(FrameResult::Dropped)
                }
            }
        }
    }
}

macro_rules! frame {
    ($frame:expr) => {
        FrameResult::Frame(Box::new($frame))
    };
}

// Do not try to format this, the match blocks will expand heavily and result
// in a bunch of wasted space.
#[rustfmt::skip]
impl DefaultFrameParser {
    // Internal macro for quickly generating a FrameResult

    fn match_frame_v2<'a>(
        &self,
        tag_header: &TagHeader,
        frame_id: [u8; 3],
        mut stream: BufStream<'a>
    ) -> ParseResult<FrameResult<'a>> {
        let frame = match frame_id.as_ref() { 
            // AttatchedPictureFrame is subtly different in ID3v2.2, so we handle it separately.
            b"PIC" => frame!(AttachedPictureFrame::parse_v2(&mut stream)?),

            _ => {
                // Convert ID3v2.2 frame IDs to their ID3v2.3 analogues, as this preserves the most frames.
                match compat::upgrade_v2_id(&frame_id) {
                    Ok(v3_id) => self.match_frame_v3(tag_header, v3_id, stream)?,
                    Err(_) => FrameResult::Unknown(FrameData::Legacy(frame_id, stream)),
                }
            }
        };

        Ok(frame)
    }

    fn match_frame_v3<'a>(
        &self,
        tag_header: &TagHeader,
        frame_id: FrameId,
        mut stream: BufStream<'a>,
    ) -> ParseResult<FrameResult<'a>> {
        let frame = match frame_id.as_ref() {
            // Involved People List
            b"IPLS" => frame!(CreditsFrame::parse(frame_id, &mut stream)?),
            // Relative volume adjustment [Frames 4.12]
            b"RVAD" => frame!(RelativeVolumeFrame::parse(&mut stream)?),
            // Equalization [Frames 4.13]
            b"EQUA" => frame!(EqualizationFrame::parse(&mut stream)?),
            // Not version-specific, go down to general frames
            _ => self.match_frame(tag_header, frame_id, stream)?,
        };

        Ok(frame)
    }

    fn match_frame_v4<'a>(
        &self,
        tag_header: &TagHeader,
        frame_id: FrameId,
        mut stream: BufStream<'a>,
    ) -> ParseResult<FrameResult<'a>> {
        // Parse ID3v2.4-specific frames.
        let frame = match frame_id.as_ref() {
            // Involved People List & Musician Credits List [Frames 4.2.2]
            b"TIPL" | b"TMCL" => frame!(CreditsFrame::parse(frame_id, &mut stream)?),
            // Relative Volume Adjustment 2 [Frames 4.11]
            b"RVA2" => frame!(RelativeVolumeFrame2::parse(&mut stream)?),
            // Equalization 2 [Frames 4.12]
            b"EQU2" => frame!(EqualizationFrame2::parse(&mut stream)?),
            // Signature Frame [Frames 4.28]
            // b"SIGN" => todo!(),
            // Seek frame [Frames 4.27]
            // b"SEEK" => todo!(),
            // Audio seek point index [Frames 4.30]
            // b"ASPI" => todo!(),
            // Not version-specific, go down to general frames
            _ => self.match_frame(tag_header, frame_id, stream)?,
        };

        Ok(frame)
    }

    fn match_frame<'a>(
        &self,
        tag_header: &TagHeader,
        frame_id: FrameId,
        mut stream: BufStream<'a>,
    ) -> ParseResult<FrameResult<'a>> {
        let frame = match frame_id.as_ref() {
            // Unique File Identifier [Frames 4.1]
            b"UFID" => frame!(FileIdFrame::parse(&mut stream)?),
            // Generic Text Information [Frames 4.2]
            _ if TextFrame::is_id(frame_id) => frame!(TextFrame::parse(frame_id, &mut stream)?),
            // User-Defined Text Information [Frames 4.2.6]
            b"TXXX" => frame!(UserTextFrame::parse(&mut stream)?),
            // Generic URL Link [Frames 4.3] 
            _ if UrlFrame::is_id(frame_id) => frame!(UrlFrame::parse(frame_id, &mut stream)?),
            // User-Defined URL Link [Frames 4.3.2]
            b"WXXX" => frame!(UserUrlFrame::parse(&mut stream)?),
            // Music CD Identifier [Frames 4.4]
            b"MCDI" => frame!(MusicCdIdFrame::parse(&mut stream)?),
            // Event timing codes [Frames 4.5]
            b"ETCO" => frame!(EventTimingCodesFrame::parse(&mut stream)?),
            // MPEG Lookup Codes [Frames 4.6]
            // b"MLLT" => todo!(),
            // Synchronized tempo codes [Frames 4.7]
            // b"SYTC" => todo!(),
            // Unsynchronized Lyrics [Frames 4.8]
            b"USLT" => frame!(UnsyncLyricsFrame::parse(&mut stream)?),
            // Unsynchronized Lyrics [Frames 4.9]
            b"SYLT" => frame!(SyncedLyricsFrame::parse(&mut stream)?),
            // Comments [Frames 4.10]
            b"COMM" => frame!(CommentsFrame::parse(&mut stream)?),
            // (Frames 4.11 & 4.12 are Version-Specific)
            // Reverb [Frames 4.13]
            // b"RVRB" => todo!(),
            // Attached Picture [Frames 4.14]
            b"APIC" => frame!(AttachedPictureFrame::parse(&mut stream)?),
            // General Encapsulated Object [Frames 4.15]
            b"GEOB" => frame!(GeneralObjectFrame::parse(&mut stream)?),
            // Play Counter [Frames 4.16]
            b"PCNT" => frame!(PlayCounterFrame::parse(&mut stream)?),
            // Popularimeter [Frames 4.17]
            b"POPM" => frame!(PopularimeterFrame::parse(&mut stream)?),
            // Relative buffer size [Frames 4.18]
            // b"RBUF" => todo!(),
            // Audio Encryption [Frames 4.19]
            // b"AENC" => todo!(),
            // Linked Information [Frames 4.20]
            // b"LINK" => todo!(),
            // Position synchronization frame [Frames 4.21]
            // b"POSS" => todo!(),
            // Terms of use frame [Frames 4.22]
            b"USER" => frame!(TermsOfUseFrame::parse(&mut stream)?),
            // Ownership frame [Frames 4.23]
            b"OWNE" => frame!(OwnershipFrame::parse(&mut stream)?),
            // Commercial frame [Frames 4.24]
            b"COMR" => frame!(CommercialFrame::parse(&mut stream)?),
            // Encryption Registration [Frames 4.25]
            // b"ENCR" => todo!(),
            // Group Identification [Frames 4.26]
            // b"GRID" => todo!(),
            // Private Frame [Frames 4.27]
            b"PRIV" => frame!(PrivateFrame::parse(&mut stream)?),
            // (Frames 4.28 -> 4.30 are version-specific)
            // Chapter Frame [ID3v2 Chapter Frame Addendum 3.1]
            b"CHAP" => frame!(ChapterFrame::parse(tag_header, &mut stream, self)?),
            // Table of Contents Frame [ID3v2 Chapter Frame Addendum 3.2]
            b"CTOC" => frame!(TableOfContentsFrame::parse(tag_header, &mut stream, self)?),
            // iTunes Podcast Frame
            b"PCST" => frame!(PodcastFrame::parse(&mut stream)?),
            // No idea, return an unknown frame
            _ => FrameResult::Unknown(FrameData::Normal(frame_id, stream)),
        };

        Ok(frame)
    }    
}

impl Default for DefaultFrameParser {
    fn default() -> Self {
        Self { strict: true }
    }
}

// Internal analogue to FrameResult that returns unknown frames.
#[derive(Debug)]
pub(crate) enum ParsedFrame {
    Frame(Box<dyn Frame>),
    Unknown(UnknownFrame),
    Dropped,
}

impl From<FrameResult<'_>> for ParsedFrame {
    fn from(other: FrameResult) -> ParsedFrame {
        match other {
            FrameResult::Frame(frame) => Self::Frame(frame),
            FrameResult::Unknown(data) => Self::Unknown(UnknownFrame::new(data, 0)),
            FrameResult::Dropped => ParsedFrame::Dropped,
        }
    }
}

pub(crate) fn parse(
    tag_header: &TagHeader,
    stream: &mut BufStream,
    parser: &impl FrameParser,
) -> ParseResult<ParsedFrame> {
    // Frame structure differs quite significantly across versions, so we have to
    // handle them separately.
    match tag_header.version() {
        Version::V22 => parse_frame_v2(tag_header, stream, parser),
        Version::V23 => parse_frame_v3(tag_header, stream, parser),
        Version::V24 => parse_frame_v4(tag_header, stream, parser),
    }
}

fn parse_frame_v2(
    tag_header: &TagHeader,
    stream: &mut BufStream,
    parser: &impl FrameParser,
) -> ParseResult<ParsedFrame> {
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
    let stream = stream.slice_stream(size)?;

    Ok(ParsedFrame::from(
        parser.parse(tag_header, FrameData::Legacy(frame_id, stream))?,
    ))
}

fn parse_frame_v3(
    tag_header: &TagHeader,
    stream: &mut BufStream,
    parser: &impl FrameParser,
) -> ParseResult<ParsedFrame> {
    let id_bytes = stream.read_array()?;
    let size = stream.read_be_u32()? as usize;
    let flags = stream.read_be_u16()?;

    // Technically, the spec says that empty frames should be a sign of a malformed tag, but they're
    // so common to the point where we should just skip them so other frames can be found.
    if size == 0 {
        return Ok(ParsedFrame::Dropped);
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

                return Ok(ParsedFrame::from(
                    parser.parse(tag_header, FrameData::Legacy(v2_id, stream))?,
                ));
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
        return Ok(ParsedFrame::Unknown(UnknownFrame::new(
            FrameData::Normal(frame_id, stream),
            flags,
        )));
    }

    // Frame-specific compression. This flag also adds a data length indicator that we will skip.
    if flags & 0x80 != 0 {
        stream.skip(4)?;

        decoded = match inflate_frame(&mut stream) {
            Ok(stream) => stream,
            Err(_) => {
                return Ok(ParsedFrame::Unknown(UnknownFrame::new(
                    FrameData::Normal(frame_id, stream),
                    flags,
                )))
            }
        };

        stream = BufStream::new(&decoded);
    }

    // Frame grouping. Pretty much nobody uses this, so its ignored.
    if flags & 0x20 != 0 && stream.len() >= 4 {
        stream.skip(1)?;
    }

    return Ok(ParsedFrame::from(
        parser.parse(tag_header, FrameData::Normal(frame_id, stream))?,
    ));
}

fn parse_frame_v4(
    tag_header: &TagHeader,
    stream: &mut BufStream,
    parser: &impl FrameParser,
) -> ParseResult<ParsedFrame> {
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

    let flags = stream.read_be_u16()?;

    // Technically, the spec says that empty frames should be a sign of a malformed tag, but they're
    // so common to the point where we should just skip them so other frames can be found.
    if size == 0 {
        return Ok(ParsedFrame::Dropped);
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
        return Ok(ParsedFrame::Unknown(UnknownFrame::new(
            FrameData::Normal(frame_id, stream),
            flags,
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
                return Ok(ParsedFrame::Unknown(UnknownFrame::new(
                    FrameData::Normal(frame_id, stream),
                    flags,
                )))
            }
        };

        stream = BufStream::new(&decoded);
    }

    return Ok(ParsedFrame::from(
        parser.parse(tag_header, FrameData::Normal(frame_id, stream))?,
    ));
}

cfg_if::cfg_if! {
    if #[cfg(feature = "id3v2_compression")] {
        fn inflate_frame(src: &mut BufStream) -> ParseResult<Vec<u8>> {
            miniz_oxide::inflate::decompress_to_vec_zlib(src.take_rest()).map_err(|err| {
                warn!("decompression failed: {:?}", err);
                ParseError::MalformedData
            })
        }
    } else {
        fn inflate_frame(src: &mut BufStream) -> ParseResult<Vec<u8>> {
            warn!("decompression is not enabled");
            Err(ParseError::Unsupported)
        }
    }
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
    fn parse_itunes_frame_sizes() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/itunes_sizes.mp3";
        let tag = Tag::open(&path).unwrap();

        assert_eq!(tag.frames["TIT2"].to_string(), "Sunshine Superman");
        assert_eq!(tag.frames["TPE1"].to_string(), "Donovan");
        assert_eq!(tag.frames["TALB"].to_string(), "Sunshine Superman");
        assert_eq!(tag.frames["TRCK"].to_string(), "1");
    }

    #[test]
    fn parse_frame_v2() {
        let frame = parse(
            &TagHeader::with_version(Version::V22),
            &mut BufStream::new(DATA_V2),
            &DefaultFrameParser { strict: true },
        )
        .unwrap();

        if let ParsedFrame::Frame(frame) = frame {
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
    fn parse_frame_v3() {
        let frame = parse(
            &TagHeader::with_version(Version::V23),
            &mut BufStream::new(DATA_V3),
            &DefaultFrameParser { strict: true },
        )
        .unwrap();

        if let ParsedFrame::Frame(frame) = frame {
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
    fn parse_frame_v4() {
        let frame = parse(
            &TagHeader::with_version(Version::V24),
            &mut BufStream::new(DATA_V4),
            &DefaultFrameParser { strict: true },
        )
        .unwrap();

        if let ParsedFrame::Frame(frame) = frame {
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
    fn parse_unknown_v2() {
        let data = b"ABC\x00\x00\x04\x16\x16\x16\x16";

        let frame = parse(
            &TagHeader::with_version(Version::V22),
            &mut BufStream::new(data),
            &DefaultFrameParser { strict: true },
        )
        .unwrap();

        if let ParsedFrame::Unknown(unknown) = frame {
            assert_eq!(unknown.id(), b"ABC");
            assert_eq!(unknown.flags(), 0);
            assert_eq!(unknown.data(), b"\x16\x16\x16\x16");

            assert!(render_unknown(&TagHeader::with_version(Version::V23), &unknown).is_empty());
        } else {
            panic!("frame is not unknown")
        }
    }

    #[test]
    fn parse_unknown_v3() {
        let data = b"APIC\x00\x00\x00\x09\x00\x60\
                     \x16\x12\x34\x56\x78\x9A\xBC\xDE\xF0";

        let frame = parse(
            &TagHeader::with_version(Version::V23),
            &mut BufStream::new(data),
            &DefaultFrameParser { strict: true },
        )
        .unwrap();

        if let ParsedFrame::Unknown(unknown) = frame {
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
    fn parse_unknown_v4() {
        let data = b"TIT2\x00\x00\x00\x0F\x00\x06\
                     \x16\x16\x16\x16\xFF\x00\xE0\x12\x34\x56\x78\x9A\xBC\xDE\xF0";

        let out = b"TIT2\x00\x00\x00\x0E\x00\x04\
                    \x16\x16\x16\x16\xFF\xE0\x12\x34\x56\x78\x9A\xBC\xDE\xF0";

        let frame = parse(
            &TagHeader::with_version(Version::V24),
            &mut BufStream::new(data),
            &DefaultFrameParser { strict: true },
        )
        .unwrap();

        if let ParsedFrame::Unknown(unknown) = frame {
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
