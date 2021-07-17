//! ID3v2 tag reading/writing.
//!
//! ID3v2 is the most common tag format, being the primary tag format in MP3 files and
//! having a presence in other formats as well. However, its also one of the most complex
//! tag formats, making this module one of the less ergonomic and more complicated APIs
//! to use in musikr.
//!
//! The ID3v2 module assumes that you have working knowledge of the ID3v2 tag format, so
//! it's recommended to read the [ID3v2.3](https://id3.org/id3v2.3.0) and
//! [ID3v2.4](https://id3.org/id3v2.4.0-structure) documents to get a better idea of the
//! tag structure.
//!
//! # Usage

pub mod collections;
mod compat;
#[macro_use]
mod macros;
pub mod frames;
mod syncdata;
pub mod tag;

use crate::core::io::BufStream;
use collections::{FrameMap, UnknownFrames};
use frames::FrameResult;
use tag::{ExtendedHeader, SaveVersion, TagHeader, Version};

use log::{info, warn};
use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::ops::Deref;
use std::io::{self, Read};
use std::path::Path;

// TODO: The current roadmap for this module:
// - Try to complete most if not all of the frame specs
// - Add further documentation
// - Work on tag upgrading
// - Add proper tag writing

#[derive(Debug, Clone)]
pub struct Tag {
    header: TagHeader,
    pub extended_header: Option<ExtendedHeader>,
    pub frames: FrameMap,
    pub unknown_frames: UnknownFrames,
}

impl Tag {
    pub fn new() -> Self {
        Self::with_version(SaveVersion::V24)
    }

    pub fn with_version(version: SaveVersion) -> Self {
        Tag {
            header: TagHeader::with_version(Version::from(version)),
            extended_header: None,
            frames: FrameMap::new(),
            unknown_frames: UnknownFrames::new(Version::from(version), Vec::new()),
        }
    }

    pub fn open<P: AsRef<Path>>(path: P) -> ParseResult<Self> {
        let mut file = File::open(path)?;

        // Read and parse the possible ID3v2 header
        let mut header_raw = [0; 10];
        file.read_exact(&mut header_raw)?;

        let mut header = TagHeader::parse(header_raw).map_err(|err| match err {
            ParseError::MalformedData => ParseError::NotFound,
            err => err,
        })?;

        // Then get the full tag data. If the size is invalid, then we will just truncate it.
        let mut tag_data = vec![0; header.size() as usize];
        let read = file.read(&mut tag_data)?;
        tag_data.truncate(read);

        let mut stream = BufStream::new(&tag_data);

        // ID3v2.3 tag-specific synchronization, decode the stream here.
        if header.version() < Version::V24 && header.flags().unsync {
            tag_data = syncdata::decode(&mut stream);
            stream = BufStream::new(&tag_data);
        }

        let mut extended_header = None;

        if header.flags().extended {
            // Certain taggers will flip the extended header flag without writing one,
            // so if parsing fails then we correct the flag.
            match ExtendedHeader::parse(&mut stream, header.version()) {
                Ok(header) => extended_header = Some(header),
                Err(_) => {
                    info!("resetting incorrectly-set extended header flag");
                    header.flags_mut().extended = false
                }
            }
        }

        // Now try parsing our frames.
        let mut frames = FrameMap::new();
        let mut unknowns = Vec::new();

        while let Ok(result) = frames::parse(&header, &mut stream) {
            match result {
                FrameResult::Frame(frame) => frames.add_boxed(frame),
                FrameResult::Unknown(unknown) => {
                    info!("found unknown frame {}", unknown.id_str());
                    unknowns.push(unknown)
                }
                FrameResult::Dropped => {
                    // Dropped frames have already moved the stream to the next
                    // frame, so we can skip them.
                }
            }
        }

        // Unknown frames are kept in a separate collection for two reasons:
        // 1. To make sure downcasting behavior is consistent
        // 2. To make sure tags of one version don't end up polluted with frames of another
        // version.
        let unknown_frames = UnknownFrames::new(header.version(), unknowns);

        Ok(Self {
            header,
            extended_header,
            frames,
            unknown_frames,
        })
    }

    pub fn version(&self) -> Version {
        self.header.version()
    }

    pub fn size(&self) -> u32 {
        self.header.size()
    }

    pub fn update(&mut self, to: SaveVersion) {
        match to {
            SaveVersion::V23 => compat::to_v3(&mut self.frames),
            SaveVersion::V24 => compat::to_v4(&mut self.frames),
        }

        // TODO: Consider updating the extended header as well.

        *self.header.version_mut() = Version::from(to);
    }

    pub fn save(&mut self) -> SaveResult<()> {
        // Before saving, ensure that our tag has been fully upgraded. ID3v2.2 tags always
        // become ID3v2.3 tags, as it has been obsoleted.
        match self.header.version() {
            Version::V22 | Version::V23 => self.update(SaveVersion::V23),
            Version::V24 => self.update(SaveVersion::V24)
        };

        // Reset all the flags that we don't really have a way to expose or support.
        // This might change in the future.
        let flags = self.header.flags_mut();
        flags.unsync = false; // Obsolete as most if not all music software is aware of ID3v2.3
        flags.extended = self.extended_header.is_some(); // Supported.
        flags.experimental = false; // This has never had a use assigned to it by the spec
        flags.footer = false; // May be exposed in the future.

        // Now we can render the tag.

        // Render the extended header first, if it's present.
        let mut body = match &self.extended_header {
            Some(ext) => ext.render(self.header.version()),
            None => Vec::new()
        };

        // Render our frames next, we don't bother writing frames considered "empty".
        // Frames that can't render are dropped, since that usually means that they
        // are too big anyway.
        for frame in self.frames.values() {
            if !frame.is_empty() {
                match frames::render(&self.header, frame.deref()) {
                    Ok(data) => body.extend(data),
                    Err(_) => warn!("could not render frame {}", frame.key())
                }
            } else {
                info!("dropping empty frame {}", frame.key())
            }
        }

        // Only render unknown frames if they line up with the current tag version.
        // 
        if self.unknown_frames.version() == self.version() {
            for frame in self.unknown_frames.frames() {
                body.extend(frames::render_unknown(&self.header, frame))
            }
        } else {
            warn!("dropping {} unknown frames", self.unknown_frames.version())
        }

        // TODO: File writing [and padding calculations]

        Ok(())
    }
}

impl Default for Tag {
    fn default() -> Self {
        Self::new()
    }
}

/// The result given after a parsing operation.
pub type ParseResult<T> = Result<T, ParseError>;

/// The error type returned when parsing ID3v2 tags.
#[derive(Debug)]
pub enum ParseError {
    /// Generic IO errors. This either means that a problem occurred while opening the file
    /// for a tag, or an unexpected EOF was encountered while parsing.
    IoError(io::Error),
    /// A part of the tag was not valid.
    MalformedData,
    /// The tag or a element of the tag is unsupported.
    Unsupported,
    /// The tag was not found in the given file.
    NotFound,
}

impl From<io::Error> for ParseError {
    fn from(other: io::Error) -> Self {
        ParseError::IoError(other)
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::IoError(err) => err.fmt(f),
            Self::MalformedData => write![f, "malformed data"],
            Self::Unsupported => write![f, "unsupported"],
            Self::NotFound => write![f, "not found"],
        }
    }
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        let error = match self {
            Self::IoError(err) => err,
            _ => return None,
        };

        Some(error)
    }
}

/// The result given after a save operation.
pub type SaveResult<T> = Result<T, SaveError>;

/// The error type returned when saving ID3v2 tags.
#[derive(Debug)]
pub enum SaveError {
    /// Generic IO errors. This means that a problem occurred while writing the tag to a file.
    IoError(io::Error),
    /// The tag [or an element in the tag] was too large to be written.
    TooLarge,
}

impl From<io::Error> for SaveError {
    fn from(other: io::Error) -> Self {
        SaveError::IoError(other)
    }
}

impl Display for SaveError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::IoError(err) => err.fmt(f),
            Self::TooLarge => write![f, "tag is too large to be saved"],
        }
    }
}

impl error::Error for SaveError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        let error = match self {
            Self::IoError(err) => err,
            _ => return None,
        };

        Some(error)
    }
}

#[cfg(test)]
mod tests {
    use crate::id3v2::frames::CommentsFrame;
    use crate::id3v2::tag::Version;
    use crate::id3v2::Tag;
    use crate::string::Encoding;
    use std::env;

    #[test]
    fn parse_id3v22() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/v22.mp3";
        let tag = Tag::open(&path).unwrap();

        assert_eq!(tag.version(), Version::V22);

        assert_eq!(tag.frames["TIT2"].to_string(), "cosmic american");
        assert_eq!(tag.frames["TPE1"].to_string(), "Anais Mitchell");
        assert_eq!(tag.frames["TALB"].to_string(), "Hymns for the Exiled");
        assert_eq!(tag.frames["TRCK"].to_string(), "3/11");
        assert_eq!(tag.frames["TYER"].to_string(), "2004");
        assert_eq!(tag.frames["TENC"].to_string(), "iTunes v4.6");

        let comm = tag.frames["COMM::eng"].downcast::<CommentsFrame>().unwrap();
        assert_eq!(comm.encoding, Encoding::Latin1);
        assert_eq!(comm.text, "Waterbug Records, www.anaismitchell.com");

        let norm = tag.frames["COMM:iTunNORM:eng"]
            .downcast::<CommentsFrame>()
            .unwrap();
        assert_eq!(norm.encoding, Encoding::Latin1);
        assert_eq!(norm.text, " 0000044E 00000061 00009B67 000044C3 00022478 00022182 00007FCC 00007E5C 0002245E 0002214E");

        let cddb = tag.frames["COMM:iTunes_CDDB_1:eng"]
            .downcast::<CommentsFrame>()
            .unwrap();
        assert_eq!(cddb.encoding, Encoding::Latin1);
        assert_eq!(cddb.text, "9D09130B+174405+11+150+14097+27391+43983+65786+84877+99399+113226+132452+146426+163829");

        let dbtk = tag.frames["COMM:iTunes_CDDB_TrackNumber:eng"]
            .downcast::<CommentsFrame>()
            .unwrap();
        assert_eq!(dbtk.encoding, Encoding::Latin1);
        assert_eq!(dbtk.text, "3");
    }
}
