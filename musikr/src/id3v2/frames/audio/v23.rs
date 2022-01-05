//! ID3v2.3-specific audio frames.

use crate::core::io::BufStream;
use crate::id3v2::frames::{Frame, FrameId};
use crate::id3v2::{ParseError, ParseResult, TagHeader};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::io;

// Stupid workaround until rust can into better pattern ranges.
// Its been 4 years. Just add half/exclusive ranges already.
const MAX_8: u64 = u8::MAX as u64;
const MAX_16: u64 = u16::MAX as u64;
const MAX_32: u64 = u32::MAX as u64;
const MAX_8_EX: u64 = MAX_8 + 1;
const MAX_16_EX: u64 = MAX_16 + 1;
const MAX_32_EX: u64 = MAX_32 + 1;

#[derive(Default, Debug, Clone)]
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
        // or even if they're floats or not. As a result, we just read plain 64-bit values. This is not
        // ideal, as it means that we can't upgrade to RVA2/EQU2 in a sane way, but its the only thing
        // we can do sadly.

        let len = usize::min((usize::from(bits) + 7) / 8, 8);

        // Since the sign of an adjustment is separate from the actual data, we will use an enum instead
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
        // Rendering this frame is not very elegant, as the way its structured requires
        // a lot of code repetition to work.

        // Set the increment decrement flags.
        let mut flags = 0;

        flags |= u8::from(self.right.volume.as_flag());
        flags |= u8::from(self.left.volume.as_flag()) * 0x2;
        flags |= u8::from(self.right_back.volume.as_flag()) * 0x4;
        flags |= u8::from(self.left_back.volume.as_flag()) * 0x8;
        flags |= u8::from(self.center.volume.as_flag()) * 0x10;
        flags |= u8::from(self.bass.volume.as_flag()) * 0x20;

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
            len = match field {
                0..=MAX_16 => 2,
                MAX_16_EX..=MAX_32 => 4,
                MAX_32_EX..=u64::MAX => 8,
            };
        }

        // Now that the length has been decided, we need to loop again and
        // actually add the rendered fields.
        let mut result = vec![flags, len * 8];

        for field in fields {
            result.extend(&field.to_be_bytes()[(8 - usize::from(len))..])
        }

        result
    }
}

impl Display for RelativeVolumeFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}R, {}L", self.right.volume, self.left.volume]
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct VolumeAdjustment {
    pub volume: Volume,
    pub peak: u64,
}

#[derive(Default, Debug, Clone)]
pub struct EqualizationFrame {
    pub adjustments: BTreeMap<Frequency, Volume>,
}

impl EqualizationFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let bits = stream.read_u8()?;

        // Bits cannot be zero.
        if bits == 0 {
            return Err(ParseError::MalformedData);
        }

        // Begin parsing our adjustments.
        let mut adjustments = BTreeMap::new();
        let len = usize::min((usize::from(bits) + 7) / 8, 8);

        while !stream.is_empty() {
            // EQUA frequencies are special in that the last bit is used as the
            // increment/decrement flag for the volume. As a result, we need
            // to clear and isolate that last bit so it can be used.
            let frequency = stream.read_be_u16()?;
            let increment = frequency & 0x8000 != 0;
            let frequency = Frequency(frequency & 0x7FFF);

            adjustments
                .entry(frequency)
                .or_insert(Volume::parse(len, increment, stream)?);
        }

        Ok(Self { adjustments })
    }
}

impl Frame for EqualizationFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"EQUA")
    }

    fn key(&self) -> String {
        String::from("EQUA")
    }

    fn is_empty(&self) -> bool {
        self.adjustments.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        let mut frequencies = Vec::new();
        let mut volumes = Vec::new();

        // Determine the minimum length we can go for alongside the setup loop.
        // Unlike RVAD, the volume fields can actually just be 1 byte.
        let mut len = 0;

        for (frequency, volume) in &self.adjustments {
            // Render the frequency, modifying the last bit to reflect the increment flag.
            // All values exceeding the allowed range are then clamped to 32767.
            let frequency = u16::min(frequency.0, 32767) | (u16::from(volume.as_flag()) * 0x8000);
            frequencies.push(frequency);
            volumes.push(volume.inner());

            len = match volume.inner() {
                0..=MAX_8 => 1,
                MAX_8_EX..=MAX_16 => 2,
                MAX_16_EX..=MAX_32 => 4,
                MAX_32_EX..=u64::MAX => 8,
            };
        }

        // No we can fully render.
        let mut result = vec![len * 8];

        for (frequency, volume) in frequencies.iter().zip(volumes.iter()) {
            result.extend(frequency.to_be_bytes());
            result.extend(&volume.to_be_bytes()[(8 - usize::from(len))..]);
        }

        result
    }
}

impl Display for EqualizationFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for (i, frequency) in self.adjustments.keys().enumerate() {
            write![f, "{}", frequency]?;

            if i < self.adjustments.len() - 1 {
                write![f, ", "]?;
            }
        }
        Ok(())
    }
}

/// The frequency of an adjustment point, in hz.
///
/// This value is written as a *15-bit* unsigned integer, allowing for a range
/// between 0 and 32767hz. All other values will be rounded to the closest valid
/// value.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Frequency(u16);

impl Display for Frequency {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The volume of an adjustment, in arbitrary units.
///
/// This value is written as a plain 64-bit unsigned integer, with the increment
/// and decrement state being written to the corresponding flag. No information is lost.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Volume {
    /// A volume increment.
    Increment(u64),
    /// A volume decrement.
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

    fn as_flag(&self) -> bool {
        matches!(self, Volume::Increment(_))
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
            stream.read_be_u64()
        }

        len if len < 8 => {
            let mut data = [0; 8];
            stream.read_exact(&mut data[8 - len..])?;
            Ok(u64::from_be_bytes(data))
        }

        _ => stream.read_be_u64(),
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

    const EQUA_DATA: &[u8] = b"EQUA\x00\x00\x00\x0D\x00\x00\
                               \x10\
                               \x00\x00\
                               \x12\x34\
                               \xAB\xCD\
                               \x00\x00\
                               \xFF\xCD\
                               \x16\x16";

    #[test]
    fn parse_rvad() {
        make_frame!(RelativeVolumeFrame, RVAD_DATA, Version::V23, frame);

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
        make_frame!(RelativeVolumeFrame, RVAD_DATA_V2, Version::V22, frame);

        assert_eq!(frame.right.volume, Volume::Decrement(0x1234));
        assert_eq!(frame.left.volume, Volume::Increment(0x0000));
        assert_eq!(frame.right.peak, 0x1616);
        assert_eq!(frame.left.peak, 0xABCD);
    }

    #[test]
    fn parse_equa() {
        make_frame!(EqualizationFrame, EQUA_DATA, Version::V23, frame);

        assert_eq!(
            frame.adjustments[&Frequency(0x7FCD)],
            Volume::Increment(0x1616)
        );
        assert_eq!(frame.adjustments[&Frequency(0)], Volume::Decrement(0x1234));
        assert_eq!(frame.adjustments[&Frequency(0x2BCD)], Volume::Increment(0));
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

        assert_render!(frame, RVAD_DATA);
    }

    #[test]
    fn render_equa() {
        let mut frame = EqualizationFrame::default();

        frame
            .adjustments
            .insert(Frequency(0x7FCD), Volume::Increment(0x1616));
        frame
            .adjustments
            .insert(Frequency(0), Volume::Decrement(0x1234));
        frame
            .adjustments
            .insert(Frequency(0x2BCD), Volume::Increment(0));

        assert_render!(frame, EQUA_DATA);
    }
}
