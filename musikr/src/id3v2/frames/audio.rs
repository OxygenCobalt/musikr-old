use crate::core::io::BufStream;
use crate::id3v2::frames::{Frame, FrameHeader, FrameId, Token};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct RelativeVolumeFrame2 {
    header: FrameHeader,
    pub desc: String,
    pub channels: BTreeMap<Channel, VolumeAdjustment>,
}

impl RelativeVolumeFrame2 {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let desc = string::read_terminated(Encoding::Latin1, stream);

        // Generally, a BTreeMap is the right tool for the job here since the maximum amount
        // of channels is small and iteration order does not matter.
        let mut channels = BTreeMap::new();

        while !stream.is_empty() {
            let channel_type = Channel::parse(stream.read_u8()?);
            let gain = Volume(stream.read_i16()? as f64 / Volume::PRECISION);

            // The ID3v2.4 spec pretty much gives NO information about how the peak volume should
            // be calculated, so this is just a shameless re-implementation of mutagens algorithm.
            // https://github.com/quodlibet/mutagen/blob/master/mutagen/id3/_specs.py#L753
            let mut peak = Peak(0.0);
            let bits = stream.read_u8()?;

            if bits != 0 {
                let peak_bytes = (bits + 7) >> 3;

                // Read a big-endian float from the amount of bytes specified
                for _ in 0..peak_bytes {
                    peak.0 *= 256.0;
                    peak.0 += stream.read_u8()? as f64;
                }

                // Since we effectively read an integer into this float, we have to normalize it into a decimal.
                let shift = ((8 - (bits & 7)) & 7) as i8 + (4 - peak_bytes as i8) * 8;
                peak.0 *= f64::powf(2.0, shift as f64);
                peak.0 /= i32::MAX as f64;
            }

            channels
                .entry(channel_type)
                .or_insert(VolumeAdjustment { gain, peak });
        }

        Ok(Self {
            header,
            desc,
            channels,
        })
    }
}

impl Frame for RelativeVolumeFrame2 {
    fn key(&self) -> String {
        format!("RVA2:{}", self.desc)
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend(string::render_terminated(Encoding::Latin1, &self.desc));

        for (&channel, adjustment) in &self.channels {
            result.push(channel as u8);
            result.extend(adjustment.gain.to_bytes());
            result.push(0x10);
            result.extend(adjustment.peak.to_bytes())
        }

        result
    }
}

impl Display for RelativeVolumeFrame2 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.desc]
    }
}

impl Default for RelativeVolumeFrame2 {
    fn default() -> Self {
        Self {
            header: FrameHeader::new(FrameId::new(b"RVA2")),
            desc: String::new(),
            channels: BTreeMap::new(),
        }
    }
}

byte_enum! {
    pub enum Channel {
        Other = 0x00,
        MasterVolume = 0x01,
        FrontRight = 0x02,
        FrontLeft = 0x03,
        BackRight = 0x04,
        BackLeft = 0x05,
        FrontCenter = 0x06,
        BackCenter = 0x07,
        Subwoofer = 0x08,
    };
    Channel::Other
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeAdjustment {
    pub gain: Volume,
    pub peak: Peak,
}

#[derive(Debug, Clone)]
pub struct EqualisationFrame2 {
    header: FrameHeader,
    pub method: InterpolationMethod,
    pub desc: String,
    pub adjustments: BTreeMap<Frequency, Volume>,
}

impl EqualisationFrame2 {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(
        header: FrameHeader,
        stream: &mut BufStream,
    ) -> ParseResult<EqualisationFrame2> {
        let method = InterpolationMethod::parse(stream.read_u8()?);
        let desc = string::read_terminated(Encoding::Latin1, stream);

        let mut adjustments = BTreeMap::new();

        while !stream.is_empty() {
            // A ID3v2.4 equalisation frame is effectively a map between the frequency [in 1/2hz intervals]
            // and the volume adjustment in decibels. All frequencies must be ordered and cannot be duplicates
            // of eachother. This is a good job for a BTreeMap, but comes at the cost of making float values
            // impossible to use in this map since they don't implement Ord [for good reasons]. Therefore we
            // just read the frequency as-is and don't do the same calculations we do on the other fields
            // in audio frames. This is not ideal, but is the best we can do without bringing in 5 useless
            // dependencies for fixed-point numbers. Oh well.
            let frequency = Frequency(stream.read_u16()?);
            let volume = Volume(stream.read_i16()? as f64 / Volume::PRECISION);

            adjustments.insert(frequency, volume);
        }

        Ok(EqualisationFrame2 {
            header,
            method,
            desc,
            adjustments,
        })
    }
}

impl Frame for EqualisationFrame2 {
    fn key(&self) -> String {
        format!("EQU2:{}", self.desc)
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.adjustments.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        let mut result = vec![self.method as u8];

        result.extend(string::render_terminated(Encoding::Latin1, &self.desc));

        for (frequency, &volume) in &self.adjustments {
            result.extend(frequency.to_bytes());
            result.extend(volume.to_bytes());
        }

        result
    }
}

impl Display for EqualisationFrame2 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.desc]
    }
}

impl Default for EqualisationFrame2 {
    fn default() -> Self {
        Self {
            header: FrameHeader::new(FrameId::new(b"EQU2")),
            method: InterpolationMethod::default(),
            desc: String::new(),
            adjustments: BTreeMap::new(),
        }
    }
}

byte_enum! {
    pub enum InterpolationMethod {
        Band = 0x00,
        Linear = 0x01,
    };
    InterpolationMethod::Band
}

impl Default for InterpolationMethod {
    fn default() -> Self {
        InterpolationMethod::Linear
    }
}

/// The volume of an adjustment in decibels.
///
/// This value is written as a i16 representing the volume * 512, allowing for a range
/// of +/- 64 Db with a precision of 0.001953125 dB. All values outside of this range
/// will be rounded to the closest valid value.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Volume(pub f64);

impl Volume {
    const PRECISION: f64 = 512.0;

    fn to_bytes(self) -> [u8; 2] {
        ((self.0 * Self::PRECISION)
            .clamp(i16::MIN as f64, i16::MAX as f64)
            .round() as i16)
            .to_be_bytes()
    }
}

impl Display for Volume {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The peak volume of an adjustment, in decibels.
///
/// This value is written as a u16 representing the volume * 32768, allowing for a range
/// of 0-2 Db with a precision of 0.000030517578125 dB. All values outside of this range
/// will be rounded to the closest valid value.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Peak(pub f64);

impl Peak {
    const PRECISION: f64 = 32768.0;

    fn to_bytes(self) -> [u8; 2] {
        // The peak can theoretically be infinite, but we cap it to a u16 for simplicity.
        ((self.0 * Self::PRECISION)
            .clamp(0.0, u16::MAX as f64)
            .round() as u16)
            .to_be_bytes()
    }
}

impl Display for Peak {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The frequency of an adjustment point, in hz.
///
/// This value encodes a frequency as a u16 in 0.5hz intervals, allowing for
/// a range between 0hz and 32767hz.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Frequency(pub u16);

impl Frequency {
    fn to_bytes(self) -> [u8; 2] {
        self.0.to_be_bytes()
    }
}

impl Display for Frequency {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::tag::Version;

    const RVA2_DATA: &[u8] = b"Description\0\
                              \x01\xfb\x8c\x10\x12\x23\
                              \x08\x04\x01\x10\x00\x00";

    const RVA2_WEIRD: &[u8] = b"Description\0\
                              \x02\xfb\x8c\x24\x01\x22\x30\x00\x00\
                              \x03\x04\x01\x00";

    const EQU2_DATA: &[u8] = b"\x01Description\0\x01\x01\x04\x00\x16\x16\x10\x08";

    #[test]
    fn parse_rva2() {
        let frame = RelativeVolumeFrame2::parse(
            FrameHeader::new(FrameId::new(b"RVA2")),
            &mut BufStream::new(RVA2_DATA),
        )
        .unwrap();

        assert_eq!(frame.desc, "Description");

        let master = &frame.channels[&Channel::MasterVolume];
        assert_eq!(master.gain, Volume(-2.2265625));
        assert_eq!(master.peak, Peak(0.141693115300356));

        let front_left = &frame.channels[&Channel::Subwoofer];
        assert_eq!(front_left.gain, Volume(2.001953125));
        assert_eq!(front_left.peak, Peak(0.0));
    }

    #[test]
    fn parse_weird_rva2() {
        let frame = RelativeVolumeFrame2::parse(
            FrameHeader::new(FrameId::new(b"RVA2")),
            &mut BufStream::new(RVA2_WEIRD),
        )
        .unwrap();

        assert_eq!(frame.desc, "Description");

        // Test weird bit-padded peaks
        let front_right = &frame.channels[&Channel::FrontRight];
        assert_eq!(front_right.gain, Volume(-2.2265625));
        assert_eq!(front_right.peak, Peak(0.141693115300356));

        // Test absent peaks
        let front_left = &frame.channels[&Channel::FrontLeft];
        assert_eq!(front_left.gain, Volume(2.001953125));
        assert_eq!(front_left.peak, Peak(0.0));
    }

    #[test]
    fn render_rva2() {
        let mut frame = RelativeVolumeFrame2::new();
        frame.desc.push_str("Description");

        frame.channels.insert(
            Channel::MasterVolume,
            VolumeAdjustment {
                gain: Volume(-2.2265625),
                peak: Peak(0.141693115300356),
            },
        );

        frame.channels.insert(
            Channel::Subwoofer,
            VolumeAdjustment {
                gain: Volume(2.001953125),
                peak: Peak(0.0),
            },
        );

        assert_eq!(
            frame.render(&TagHeader::with_version(Version::V24)),
            RVA2_DATA
        );
    }

    #[test]
    fn parse_equ2() {
        let frame = EqualisationFrame2::parse(
            FrameHeader::new(FrameId::new(b"EQU2")),
            &mut BufStream::new(EQU2_DATA),
        )
        .unwrap();

        assert_eq!(frame.desc, "Description");
        assert_eq!(frame.adjustments[&Frequency(257)], Volume(2.0));
        assert_eq!(frame.adjustments[&Frequency(5654)], Volume(8.015625));
    }

    #[test]
    fn render_equ2() {
        let mut frame = EqualisationFrame2::new();
        frame.desc.push_str("Description");
        frame.adjustments.insert(Frequency(257), Volume(2.0));
        frame.adjustments.insert(Frequency(5654), Volume(8.015625));

        assert_eq!(
            frame.render(&TagHeader::with_version(Version::V24)),
            EQU2_DATA
        );
    }
}
