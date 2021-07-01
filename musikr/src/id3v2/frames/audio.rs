use crate::core::io::BufStream;
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader, Token};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use indexmap::IndexMap;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

// Recast existing maxes as floats for simplicity
const I32_MAX: f64 = i32::MAX as f64;
const I16_MIN: f64 = i16::MIN as f64;
const I16_MAX: f64 = i16::MAX as f64;
const U16_MAX: f64 = u16::MAX as f64;

const VOLUME_PRECISION: f64 = 512.0;
const PEAK_PRECISION: f64 = 32768.0;

pub struct RelativeVolumeFrame2 {
    header: FrameHeader,
    desc: String,
    channels: IndexMap<Channel, VolumeAdjustment>,
}

impl RelativeVolumeFrame2 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags(b"RVA2", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        Self {
            header,
            desc: String::new(),
            channels: IndexMap::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let desc = string::read_terminated(Encoding::Latin1, stream);
        let mut channels = IndexMap::new();

        while !stream.is_empty() {
            let channel_type = Channel::parse(stream.read_u8()?);

            // The gain is encoded as a 16-bit signed integer representing the
            // adjustment * 512. Convert it to a float and then divide it to get
            // the true value.
            let gain = stream.read_i16()? as f64 / VOLUME_PRECISION;

            // The ID3v2.4 spec pretty much gives NO information about how the peak volume should
            // be calculated, so this is just a shameless re-implementation of mutagens algorithm.
            // https://github.com/quodlibet/mutagen/blob/master/mutagen/id3/_specs.py#L753
            let mut peak = 0.0;
            let bits = stream.read_u8()?;

            if bits != 0 {
                let peak_bytes = (bits + 7) >> 3;

                // Read a big-endian float from the amount of bytes specified
                for _ in 0..peak_bytes {
                    peak *= 256.0;
                    peak += stream.read_u8()? as f64;
                }

                // Since we effectively read an integer into this float, we have to normalize it into a decimal.
                let shift = ((8 - (bits & 7)) & 7) as i8 + (4 - peak_bytes as i8) * 8;
                peak *= f64::powf(2.0, shift as f64);
                peak /= I32_MAX;
            }

            channels
                .entry(channel_type)
                .or_insert(VolumeAdjustment { gain, peak });
        }

        Ok(RelativeVolumeFrame2 {
            header,
            desc,
            channels,
        })
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn channels(&self) -> &IndexMap<Channel, VolumeAdjustment> {
        &self.channels
    }

    pub fn desc_mut(&mut self) -> &mut String {
        &mut self.desc
    }

    pub fn channels_mut(&mut self) -> &mut IndexMap<Channel, VolumeAdjustment> {
        &mut self.channels
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

            let gain = encode_volume(adjustment.gain);
            result.extend(gain.to_be_bytes());

            let peak = encode_peak(adjustment.peak);
            result.push(0x10);
            result.extend(peak.to_be_bytes())
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
        Self::with_flags(FrameFlags::default())
    }
}

byte_enum! {
    #[derive(Hash)]
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

pub struct VolumeAdjustment {
    pub gain: f64,
    pub peak: f64,
}

pub struct EqualisationFrame2 {
    header: FrameHeader,
    method: InterpolationMethod,
    desc: String,
    adjustments: BTreeMap<u16, f64>
}

impl EqualisationFrame2 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags(b"EQU2", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        Self {
            header,
            method: InterpolationMethod::default(),
            desc: String::new(),
            adjustments: BTreeMap::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<EqualisationFrame2> {
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
            let frequency = stream.read_u16()?;
            let volume = stream.read_i16()? as f64 / VOLUME_PRECISION;

            adjustments.insert(frequency, volume);
        }

        Ok(EqualisationFrame2 {
            header,
            method,
            desc,
            adjustments
        })
    }

    pub fn method(&self) -> InterpolationMethod {
        self.method
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn adjustments(&self) -> &BTreeMap<u16, f64> {
        &self.adjustments
    }

    pub fn method_mut(&mut self) -> &mut InterpolationMethod {
        &mut self.method
    }

    pub fn desc_mut(&mut self) -> &mut String {
        &mut self.desc
    }

    pub fn adjustments_mut(&mut self) -> &mut BTreeMap<u16, f64> {
        &mut self.adjustments
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
            result.extend(frequency.to_be_bytes());
            result.extend(encode_volume(volume).to_be_bytes());
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
        Self::with_flags(FrameFlags::default())
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

fn encode_volume(volume: f64) -> i16 {
    // All volume fields are restricted to 16 bits, so we clamp it as such
    (volume * VOLUME_PRECISION).clamp(I16_MIN, I16_MAX).round() as i16
}

fn encode_peak(peak: f64) -> u16 {
    // The peak can theoretically be infinite, but we cap it to a u16 for simplicity.
    (peak * PEAK_PRECISION).clamp(0.0, U16_MAX).round() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    const RVA2_DATA: &[u8] = b"Description\0\
                              \x01\xfb\x8c\x10\x12\x23\
                              \x08\x04\x01\x10\x00\x00";

    const RVA2_WEIRD: &[u8] = b"Description\0\
                              \x02\xfb\x8c\x24\x01\x22\x30\x00\x00\
                              \x03\x04\x01\x00";

    const EQU2_DATA: &[u8] = b"\x01Description\0\x01\x01\x04\x00\x16\x16\x10\x08";

    #[test]
    fn parse_rva2() {
        let frame =
            RelativeVolumeFrame2::parse(FrameHeader::new(b"RVA2"), &mut BufStream::new(RVA2_DATA))
                .unwrap();

        assert_eq!(frame.desc(), "Description");

        // Test Normal peak
        let master = &frame.channels()[&Channel::MasterVolume];
        assert_eq!(master.gain, -2.2265625);
        assert_eq!(master.peak, 0.141693115300356);

        // Test channels with no peaks
        let front_left = &frame.channels()[&Channel::Subwoofer];
        assert_eq!(front_left.gain, 2.001953125);
        assert_eq!(front_left.peak, 0.0);
    }

    #[test]
    fn parse_weird_rva2() {
        let frame =
            RelativeVolumeFrame2::parse(FrameHeader::new(b"RVA2"), &mut BufStream::new(RVA2_WEIRD))
                .unwrap();

        assert_eq!(frame.desc(), "Description");

        // Test weird bit-padded peaks
        let front_right = &frame.channels()[&Channel::FrontRight];
        assert_eq!(front_right.gain, -2.2265625);
        assert_eq!(front_right.peak, 0.141693115300356);

        // Test absent peaks
        let front_left = &frame.channels()[&Channel::FrontLeft];
        assert_eq!(front_left.gain, 2.001953125);
        assert_eq!(front_left.peak, 0.0);
    }

    #[test]
    fn render_rva2() {
        let mut frame = RelativeVolumeFrame2::new();
        frame.desc_mut().push_str("Description");

        let channels = frame.channels_mut();

        channels.insert(
            Channel::MasterVolume,
            VolumeAdjustment {
                gain: -2.2265625,
                peak: 0.141693115300356,
            },
        );

        channels.insert(
            Channel::Subwoofer,
            VolumeAdjustment {
                gain: 2.001953125,
                peak: 0.0,
            },
        );

        assert_eq!(frame.render(&TagHeader::with_version(4)), RVA2_DATA);
    }

    #[test]
    fn parse_equ2() {
        let frame = EqualisationFrame2::parse(FrameHeader::new(b"EQU2"), &mut BufStream::new(EQU2_DATA)).unwrap();
        let adjustments = frame.adjustments();
        
        assert_eq!(frame.desc(), "Description");

        assert_eq!(adjustments[&257], 2.0);
        assert_eq!(adjustments[&5654], 8.015625);
    }

    #[test]
    fn render_equ2() {
        let mut frame = EqualisationFrame2::new();
        frame.desc_mut().push_str("Description");
        
        let adjustments = frame.adjustments_mut();
        adjustments.insert(257, 2.0);
        adjustments.insert(5654, 8.015625);

        assert_eq!(frame.render(&TagHeader::with_version(4)), EQU2_DATA);   
    }
}
