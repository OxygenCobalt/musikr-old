use crate::core::io::BufStream;
use crate::id3v2::frames::{Frame, FrameId};
use crate::id3v2::{ParseError, ParseResult, TagHeader};
use std::fmt::{self, Display, Formatter};
use std::io;

#[derive(Debug, Clone)]
pub struct RelativeVolumeFrame {
    pub right: VolumeAdjustment,
    pub left: VolumeAdjustment,
    pub right_back: VolumeAdjustment,
    pub left_back: VolumeAdjustment,
    pub center: VolumeAdjustment,
    pub bass: VolumeAdjustment,
}

impl RelativeVolumeFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let mut frame = Self::default();

        let flags = stream.read_u8()?;
        let bits = stream.read_u8()?;

        if bits == 0 {
            // Fields must have at least 1 bit.
            return Err(ParseError::MalformedData);
        }

        // Once again, the spec says NOTHING about what units the volume fields are supposed to represent,
        // or even if they're floats or not. As a result, we just read plain 64-bit values,

        let len = usize::min((bits as usize + 7) / 8, 4);

        // Since the sign of an adjustment is seperate from the actual data, we will use an enum instead
        // of a signed integer so that we don't lose information.

        // Left/Right volume fields. These are mandatory.
        frame.right.volume = Volume::parse(len, flags & 0x1 != 0, stream)?;
        frame.left.volume = Volume::parse(len, flags & 0x2 != 0, stream)?;

        // The rest of the fields are optional/ID3v2.3-specific, so if they're not present we zero them.

        // Left/Right peak values.
        frame.right.peak = read_n_u64(len, stream).unwrap_or_default();
        frame.left.peak = read_n_u64(len, stream).unwrap_or_default();

        // Left/Right volume fields
        frame.right_back.volume = Volume::parse(len, flags & 0x4 != 0, stream).unwrap_or_default();
        frame.left_back.volume = Volume::parse(len, flags & 0x8 != 0, stream).unwrap_or_default();

        // Back left/back right peak fields
        frame.right_back.peak = read_n_u64(len, stream).unwrap_or_default();
        frame.left_back.peak = read_n_u64(len, stream).unwrap_or_default();

        // Center volume/peak
        frame.center.volume = Volume::parse(len, flags & 0x10 != 0, stream).unwrap_or_default();
        frame.center.peak = read_n_u64(len, stream).unwrap_or_default();

        // Center back volume/peak
        frame.bass.volume = Volume::parse(len, flags & 0x20 != 0, stream).unwrap_or_default();
        frame.bass.peak = read_n_u64(len, stream).unwrap_or_default();

        Ok(frame)
    }
}

impl Frame for RelativeVolumeFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"RVAD")
    }

    fn key(&self) -> String {
        String::from("RVAD")
    }

    fn is_empty(&self) -> bool {
        // Frame is never empty
        false
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        // Rendering this frame is...not very elegant, as the way its structured makes
        // normal list comprehension quite difficult. Get read for alot of duplicate code

        // Get increment/decrement flags in order
        let mut flags = 0;

        flags |= u8::from(matches!(self.right.volume, Volume::Increment(_)));
        flags |= u8::from(matches!(self.left.volume, Volume::Increment(_))) * 0x2;
        flags |= u8::from(matches!(self.right_back.volume, Volume::Increment(_))) * 0x4;
        flags |= u8::from(matches!(self.left_back.volume, Volume::Increment(_))) * 0x8;
        flags |= u8::from(matches!(self.center.volume, Volume::Increment(_))) * 0x10;
        flags |= u8::from(matches!(self.bass.volume, Volume::Increment(_))) * 0x20;

        // Get the fields in order. We render them all for simplicity.
        let fields = [
            self.right.volume.inner(),
            self.left.volume.inner(),
            self.right.peak,
            self.left.peak,
            self.right_back.volume.inner(),
            self.left_back.volume.inner(),
            self.right_back.peak,
            self.left_back.peak,
            self.center.volume.inner(),
            self.center.peak,
            self.bass.volume.inner(),
            self.bass.peak,
        ];

        // Normalize the length of these items, the minimum being 16 bits.
        let mut len = 4;

        for field in fields {
            // Stupid workaround until rust can into exclusive pattern ranges.
            // It has been 4 years. Add this already.

            const MAX_16: u64 = u16::MAX as u64;
            const MAX_32: u64 = u32::MAX as u64;
            const MAX_16_EX: u64 = MAX_16 + 1;
            const MAX_32_EX: u64 = MAX_32 + 1;

            len = match field {
                0..=MAX_16 => 2,
                MAX_16_EX..=MAX_32 => 4,
                MAX_32_EX..=u64::MAX => 8
            };
        }

        // Now that the length has been decided, we need to loop again and
        // actually add the rendered fields.
        let mut result = vec![flags, len * 8];

        for field in fields {
            result.extend(&field.to_be_bytes()[(8 - len as usize)..])
        }

        result
    }
}

impl Display for RelativeVolumeFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}R, {}L", self.right.volume, self.left.volume]
    }
}

impl Default for RelativeVolumeFrame {
    fn default() -> Self {
        Self {
            right: VolumeAdjustment::default(),
            left: VolumeAdjustment::default(),
            right_back: VolumeAdjustment::default(),
            left_back: VolumeAdjustment::default(),
            center: VolumeAdjustment::default(),
            bass: VolumeAdjustment::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct VolumeAdjustment {
    pub volume: Volume,
    pub peak: u64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Volume {
    Increment(u64),
    Decrement(u64),
}

impl Volume {
    fn parse(len: usize, increment: bool, stream: &mut BufStream) -> ParseResult<Self> {
        let volume = read_n_u64(len, stream)?;

        if increment {
            Ok(Self::Increment(volume))
        } else {
            Ok(Self::Decrement(volume))
        }
    }

    fn inner(&self) -> u64 {
        match self {
            Self::Increment(val) => *val,
            Self::Decrement(val) => *val,
        }
    }
}

impl Display for Volume {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Increment(val) => write![f, "{}", val],
            Self::Decrement(val) => write![f, "-{}", val],
        }
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self::Decrement(0)
    }
}

fn read_n_u64(len: usize, stream: &mut BufStream) -> io::Result<u64> {
    match len {
        len if len > 8 => {
            stream.skip(len - 8)?;
            stream.read_u64()
        }

        len if len < 8 => {
            let mut data = [0; 8];
            stream.read_exact(&mut data[8 - len..])?;
            Ok(u64::from_be_bytes(data))
        }

        _ => stream.read_u64(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::tag::Version;

    const RVAD_DATA: &[u8] = b"RVAD\x00\x00\x00\x32\x00\x00\
                              \x2d\x20\
                              \xAB\xCD\xEF\x16\
                              \x01\x02\x04\x08\
                              \x16\x16\x16\x16\
                              \x00\x00\x00\x00\
                              \x00\xFF\x00\xFF\
                              \xFF\x00\xFF\x00\
                              \x61\xFE\xDC\xBA\
                              \x20\x40\x80\x00\
                              \x00\x00\x00\x00\
                              \x00\x00\x10\x10\
                              \x4F\x58\x43\x42\
                              \xFF\xFF\xFF\xFF";

    const RVAD_DATA_V2: &[u8] = b"RVA\x00\x00\x0A\
                                  \x02\x10\
                                  \x12\x34\
                                  \x00\x00\
                                  \x16\x16\
                                  \xAB\xCD";

    #[test]
    fn parse_rvad() {
        crate::make_frame!(RelativeVolumeFrame, RVAD_DATA, Version::V23, frame);

        assert_eq!(frame.right.volume, Volume::Increment(0xABCDEF16));
        assert_eq!(frame.left.volume, Volume::Decrement(0x01020408));
        assert_eq!(frame.right.peak, 0x16161616);
        assert_eq!(frame.left.peak, 0);

        assert_eq!(frame.right_back.volume, Volume::Increment(0x00FF00FF));
        assert_eq!(frame.left_back.volume, Volume::Increment(0xFF00FF00));
        assert_eq!(frame.right_back.peak, 0x61FEDCBA);
        assert_eq!(frame.left_back.peak, 0x20408000);

        assert_eq!(frame.center.volume, Volume::Decrement(0));
        assert_eq!(frame.center.peak, 0x1010);

        assert_eq!(frame.bass.volume, Volume::Increment(0x4F584342));
        assert_eq!(frame.bass.peak, 0xFFFFFFFF);
    }

    #[test]
    fn parse_rvad_v2() {
        crate::make_frame!(RelativeVolumeFrame, RVAD_DATA_V2, Version::V22, frame);

        assert_eq!(frame.right.volume, Volume::Decrement(0x1234));
        assert_eq!(frame.left.volume, Volume::Increment(0x0000));
        assert_eq!(frame.right.peak, 0x1616);
        assert_eq!(frame.left.peak, 0xABCD);
    }

    #[test]
    fn render_rvad() {
        let frame = RelativeVolumeFrame {
            right: VolumeAdjustment {
                volume: Volume::Increment(0xABCDEF16),
                peak: 0x16161616,
            },
            left: VolumeAdjustment {
                volume: Volume::Decrement(0x01020408),
                peak: 0,
            },
            right_back: VolumeAdjustment {
                volume: Volume::Increment(0x00FF00FF),
                peak: 0x61FEDCBA,
            },
            left_back: VolumeAdjustment {
                volume: Volume::Increment(0xFF00FF00),
                peak: 0x20408000,
            },
            center: VolumeAdjustment {
                volume: Volume::Decrement(0),
                peak: 0x1010,
            },
            bass: VolumeAdjustment {
                volume: Volume::Increment(0x4F584342),
                peak: 0xFFFFFFFF,
            },
        };

        crate::assert_render!(frame, RVAD_DATA);
    }
}
