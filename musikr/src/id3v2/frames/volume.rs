use crate::core::io::BufStream;
use crate::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader, Token};
use crate::id3v2::{ParseResult, TagHeader};
use std::fmt::{self, Display, Formatter};
use indexmap::IndexMap;

// Recast existing maxes as floats so we can use them
const I32_MAX: f64 = i32::MAX as f64;
const I16_MIN: f64 = i16::MIN as f64;
const I16_MAX: f64 = i16::MAX as f64;

const GAIN_PRECISION: f64 = 512.0;
const PEAK_PRECISION: f64 = 32768.0;

pub struct RelativeVolumeFrame2 {
    header: FrameHeader,
    desc: String,
    channels: IndexMap<Channel, Adjustment>
}

impl RelativeVolumeFrame2 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags(b"RVA2", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        RelativeVolumeFrame2 {
            header,
            desc: String::new(),
            channels: IndexMap::new()
        }
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let desc = string::read_terminated(Encoding::Latin1, stream);
        let mut channels = IndexMap::new();

        while !stream.is_empty() {
            let channel_type = Channel::new(stream.read_u8()?);

            // The gain is the value in decibels * 512.
            // f32 is used here since the extra precision really isnt needed.
            let gain = stream.read_i16()? as f64 / GAIN_PRECISION;

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

            channels.entry(channel_type).or_insert(
                Adjustment { gain, peak }
            );
        }

        Ok(RelativeVolumeFrame2 {
            header,
            desc,
            channels
        })
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn channels(&self) -> &IndexMap<Channel, Adjustment> {
        &self.channels
    }

    pub fn desc_mut(&mut self) -> &mut String {
        &mut self.desc
    }

    pub fn channels_mut(&mut self) -> &mut IndexMap<Channel, Adjustment> {
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
        
        for (channel, adjustment) in &self.channels {
            result.push(*channel as u8);

            // The gain is restricted to 16 bytes, so we clamp it to those limits
            let gain = (adjustment.gain * GAIN_PRECISION).clamp(I16_MIN, I16_MAX).round() as i16;
            result.extend(gain.to_be_bytes());

            // Clamp the peak to 16-bits for simplicity.
            let peak = (adjustment.peak * PEAK_PRECISION).clamp(0.0, I16_MAX).round() as u16;

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
    }
}

impl Default for Channel {
    fn default() -> Self {
        Channel::Other
    }
}

pub struct Adjustment {
    pub gain: f64,
    pub peak: f64
}

#[cfg(test)]
mod tests {
    use super::*;

    const RVA2_DATA: &[u8] = b"Description\0\
                               \x01\xfb\x8c\x10\x12\x23\
                               \x02\xfb\x8c\x24\x01\x22\x30\x00\x00\
                               \x03\x04\x01\x00";

    const RVA2_OUT: &[u8] = b"Description\0\
                              \x01\xfb\x8c\x10\x12\x23\
                              \x08\x04\x01\x10\x00\x00";

    #[test]
    fn parse_rva2() {
        let frame = RelativeVolumeFrame2::parse(FrameHeader::new(b"RVA2"), &mut BufStream::new(RVA2_DATA)).unwrap();
        assert_eq!(frame.desc(), "Description");

        // Test Normal peak
        let master = &frame.channels()[&Channel::MasterVolume];
        assert_eq!(master.gain, -2.2265625);
        assert_eq!(master.peak, 0.141693115300356);

        // Test weird bit-padded peaks
        let front_right = &frame.channels()[&Channel::FrontRight];
        assert_eq!(front_right.gain, -2.2265625);
        assert_eq!(front_right.peak, 0.141693115300356);

        // Test channels with no peaks
        let front_left = &frame.channels()[&Channel::FrontLeft];
        assert_eq!(front_left.gain, 2.001953125);
        assert_eq!(front_left.peak, 0.0);
    }

    #[test]
    fn render_rva2() {
        let mut frame = RelativeVolumeFrame2::new();

        frame.desc_mut().push_str("Description");

        let channels = frame.channels_mut();
        channels.insert(Channel::MasterVolume, Adjustment {
            gain: -2.2265625,
            peak: 0.141693115300356
        });

        channels.insert(Channel::Subwoofer, Adjustment {
            gain: 2.001953125,
            peak: 0.0
        });

        assert_eq!(frame.render(&TagHeader::with_version(4)), RVA2_OUT);
    }
}