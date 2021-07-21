use std::fmt::{self, Display, Formatter};
use std::iter::IntoIterator;
use std::str::{self, FromStr};
use std::convert::{TryInto, TryFrom};
use std::error;

byte_enum! {
    pub enum TimestampFormat {
        Other = 0x00,
        MpegFrames = 0x01,
        Millis = 0x02,
    };
    TimestampFormat::Other
}

impl Default for TimestampFormat {
    fn default() -> Self {
        TimestampFormat::Millis
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Clone, Copy)]
pub struct Language([u8; 3]);

impl Language {
    pub fn new(code: &[u8; 3]) -> Self {
        Self::try_new(code).unwrap()
    }

    pub fn try_new(code: &[u8; 3]) -> Result<Self, LangError> {
        let mut lang = [0; 3];

        for (i, byte) in code.iter().enumerate() {
            // ISO-639-2 language codes are always alphabetic ASCII chars.
            if !byte.is_ascii_alphabetic() {
                return Err(LangError(()));
            }

            // Certain taggers might write the language code as uppercase chars.
            // For simplicity, we make them lowercase.
            lang[i] = byte.to_ascii_lowercase();
        }

        Ok(Language(lang))
    }

    pub fn inner(&self) -> &[u8; 3] {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        // We've asserted that this is completely ascii, so we can unwrap
        str::from_utf8(&self.0).unwrap()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

inner_eq!(Language, [u8; 3]);
inner_eq!(Language, &'a [u8]);
inner_eq!(Language, &[u8; 3]);
inner_borrow!(Language, [u8]);
inner_index!(Language, u8);
inner_ranged_index!(Language, [u8]);

impl Display for Language {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.as_str()]
    }
}

impl Default for Language {
    fn default() -> Self {
        // Spec says that language codes should be "xxx" by default
        Language([b'x'; 3])
    }
}

impl AsRef<[u8]> for Language {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl IntoIterator for Language {
    type Item = u8;
    type IntoIter = std::array::IntoIter<u8, 3>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter::new(self.0)
    }
}

impl<'a> IntoIterator for &'a Language {
    type Item = &'a u8;
    type IntoIter = std::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromStr for Language {
    type Err = LangError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 3 {
            return Err(LangError(()));
        }

        let mut lang = [0; 3];

        for (i, ch) in s.chars().enumerate() {
            if !ch.is_ascii() {
                return Err(LangError(()));
            }

            lang[i] = ch as u8;
        }

        Ok(Self(lang))
    }
}

impl TryFrom<&[u8]> for Language {
    type Error = LangError;

    fn try_from(other: &[u8]) -> Result<Self, Self::Error> {
        match other.try_into() {
            Ok(arr) => Self::try_new(&arr),
            Err(_) => Err(LangError(()))
        }
    }
}

impl TryFrom<[u8; 3]> for Language {
    type Error = LangError;

    fn try_from(other: [u8; 3]) -> Result<Self, Self::Error> {
        Self::try_new(&other)
    }
}

impl TryFrom<&[u8; 3]> for Language {
    type Error = LangError;

    fn try_from(other: &[u8; 3]) -> Result<Self, Self::Error> {
        Self::try_new(other)
    }
}

#[derive(Debug)]
pub struct LangError(());

impl error::Error for LangError {
    // Nothing to implement
}

impl Display for LangError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "language was not a 3-byte sequence of ascii alphabetic chars"]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FrameId([u8; 4]);

impl FrameId {
    pub fn new(frame_id: &[u8; 4]) -> Self {
        Self::try_new(frame_id).unwrap()
    }

    pub fn try_new(frame_id: &[u8; 4]) -> Result<Self, FrameIdError> {
        if !Self::validate(frame_id) {
            return Err(FrameIdError(()));
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

    pub fn as_slice(&self) -> &[u8] {
        &self.0
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


inner_eq!(FrameId, [u8; 4]);
inner_eq!(FrameId, &'a [u8]);
inner_eq!(FrameId, &'a [u8; 4]);

impl AsRef<[u8]> for FrameId {
    fn as_ref(&self) -> &'_ [u8] {
        self.as_slice()
    }
}

impl Display for FrameId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.as_str()]
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

impl TryFrom<&[u8]> for FrameId {
    type Error = FrameIdError;

    fn try_from(other: &[u8]) -> Result<Self, Self::Error> {
        match other.try_into() {
            Ok(arr) => Self::try_new(&arr),
            Err(_) => Err(FrameIdError(()))
        }
    }
}

impl TryFrom<[u8; 4]> for FrameId {
    type Error = FrameIdError;

    fn try_from(other: [u8; 4]) -> Result<Self, Self::Error> {
        Self::try_new(&other)
    }
}

impl TryFrom<&[u8; 4]> for FrameId {
    type Error = FrameIdError;

    fn try_from(other: &[u8; 4]) -> Result<Self, Self::Error> {
        Self::try_new(other)
    }
}

#[derive(Debug)]
pub struct FrameIdError(());

impl error::Error for FrameIdError {
    // Nothing to implement
}

impl Display for FrameIdError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "frame id was not a 4-byte sequence of uppercase ascii alphabetic chars or digits"]
    }
}