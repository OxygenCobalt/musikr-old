//! Frames that contain files.

use crate::core::io::BufStream;
use crate::id3v2::frames::{encoding, Frame, FrameId};
use crate::id3v2::{ParseResult, TagHeader};
use crate::core::string::{self, Encoding};
use log::info;
use std::fmt::{self, Display, Formatter};

#[derive(Default, Debug, Clone)]
pub struct AttachedPictureFrame {
    pub encoding: Encoding,
    pub mime: String,
    pub desc: String,
    pub pic_type: PictureType,
    pub picture: Vec<u8>,
}

impl AttachedPictureFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;

        let mut mime = string::read_terminated(Encoding::Latin1, stream);

        // image/ is implied when there is no mime type.
        if mime.is_empty() {
            info!("found empty mime type, assuming image/");
            mime.push_str("image/");
        }

        let pic_type = PictureType::parse(stream.read_u8()?);
        let desc = string::read_terminated(encoding, stream);

        let picture = stream.take_rest().to_vec();

        Ok(Self {
            encoding,
            mime,
            desc,
            pic_type,
            picture,
        })
    }

    pub(crate) fn parse_v2(stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;

        // The main way that ID3v2.2 PIC frames differ is the presence of a 3-byte "image format"
        // instead of a MIME type. We just map PNG/JPG to image/png and image/jpeg respectively
        // while all other types map to image/.

        let mime = match &stream.read_array::<3>()? {
            b"PNG" => String::from("image/png"),
            b"JPG" => String::from("image/jpeg"),
            _ => String::from("image/"),
        };

        let pic_type = PictureType::parse(stream.read_u8()?);
        let desc = string::read_terminated(encoding, stream);

        let picture = stream.take_rest().to_vec();

        Ok(Self {
            encoding,
            mime,
            desc,
            pic_type,
            picture,
        })
    }
}

impl Frame for AttachedPictureFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"APIC")
    }

    fn key(&self) -> String {
        // *Technically* the spec says that there can only be one FileIcon and OtherFileIcon
        // APIC frame per tag, but pretty much no tagger enforces this.
        format!["APIC:{}", self.desc]
    }

    fn is_empty(&self) -> bool {
        self.picture.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.version());
        result.push(encoding::render(self.encoding));

        result.extend(string::render_terminated(Encoding::Latin1, &self.mime));
        result.push(self.pic_type as u8);
        result.extend(string::render_terminated(encoding, &self.desc));
        result.extend(self.picture.iter());

        result
    }
}

impl Display for AttachedPictureFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{} ", self.mime]?;

        if !self.desc.is_empty() {
            write![f, "\"{}\" ", self.desc]?;
        }

        write![f, "[{:?}]", self.pic_type]
    }
}

byte_enum! {
    pub enum PictureType {
        Other = 0x00,
        FileIcon = 0x01,
        OtherFileIcon = 0x02,
        FrontCover = 0x03,
        BackCover = 0x04,
        LeafletPage = 0x05,
        Media = 0x06,
        LeadArtist = 0x07,
        Artist = 0x08,
        Conductor = 0x09,
        Band = 0x0A,
        Composer = 0x0B,
        Writer = 0x0C,
        RecordingLocation = 0x0D,
        DuringRecording = 0x0E,
        DuringPerformance = 0x0F,
        MovieScreenCapture = 0x10,
        ColoredFish = 0x11,
        Illustration = 0x12,
        BandLogo = 0x13,
        PublisherLogo = 0x14,
    };
    PictureType::Other
}

impl Default for PictureType {
    fn default() -> Self {
        PictureType::FrontCover
    }
}

#[derive(Default, Debug, Clone)]
pub struct GeneralObjectFrame {
    pub encoding: Encoding,
    pub mime: String,
    pub filename: String,
    pub desc: String,
    pub data: Vec<u8>,
}

impl GeneralObjectFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let mime = string::read_terminated(Encoding::Latin1, stream);
        let filename = string::read_terminated(encoding, stream);
        let desc = string::read_terminated(encoding, stream);

        let data = stream.take_rest().to_vec();

        Ok(Self {
            encoding,
            mime,
            filename,
            desc,
            data,
        })
    }
}

impl Frame for GeneralObjectFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"GEOB")
    }

    fn key(&self) -> String {
        format!["GEOB:{}", self.desc]
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.version());
        result.push(encoding::render(self.encoding));

        result.extend(string::render_terminated(Encoding::Latin1, &self.mime));
        result.extend(string::render_terminated(encoding, &self.filename));
        result.extend(string::render_terminated(encoding, &self.desc));
        result.extend(self.data.iter());

        result
    }
}

impl Display for GeneralObjectFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if !self.mime.is_empty() {
            write![f, "{} ", self.mime]?;
        }

        if !self.filename.is_empty() {
            write![f, "\"{}\"", self.filename]?;
        }

        if !self.desc.is_empty() {
            write![f, " [{}]", self.desc]?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::tag::Version;

    const APIC_DATA: &[u8] = b"APIC\x00\x00\x00\x25\x00\x00\
                               \x00\
                               image/png\0\
                               \x03\
                               Geogaddi_Cover.png\0\
                               \x16\x16\x16\x16\x16\x16";

    const APIC_V2_DATA: &[u8] = b"PIC\x00\x00\x1e\
                                  \x00\
                                  PNG\
                                  \x03\
                                  Geogaddi_Cover.png\0\
                                  \x16\x16\x16\x16\x16\x16";

    const GEOB_DATA: &[u8] = b"GEOB\x00\x00\x00\x38\x00\x00\
                               \x01\
                               text/txt\0\
                               \xFF\xFE\x4c\x00\x79\x00\x72\x00\x69\x00\x63\x00\x73\x00\x2e\x00\x6c\x00\x72\x00\x63\x00\0\0\
                               \xFF\xFE\x4c\x00\x79\x00\x72\x00\x69\x00\x63\x00\x73\x00\0\0\
                               \x16\x16\x16\x16\x16\x16";

    #[test]
    fn parse_apic() {
        make_frame!(AttachedPictureFrame, APIC_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Latin1);
        assert_eq!(frame.mime, "image/png");
        assert_eq!(frame.pic_type, PictureType::FrontCover);
        assert_eq!(frame.desc, "Geogaddi_Cover.png");
        assert_eq!(frame.picture, b"\x16\x16\x16\x16\x16\x16");
    }

    #[test]
    fn parse_apic_v2() {
        make_frame!(AttachedPictureFrame, APIC_V2_DATA, Version::V22, frame);

        assert_eq!(frame.encoding, Encoding::Latin1);
        assert_eq!(frame.mime, "image/png");
        assert_eq!(frame.pic_type, PictureType::FrontCover);
        assert_eq!(frame.desc, "Geogaddi_Cover.png");
        assert_eq!(frame.picture, b"\x16\x16\x16\x16\x16\x16");
    }

    #[test]
    fn parse_geob() {
        make_frame!(GeneralObjectFrame, GEOB_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Utf16);
        assert_eq!(frame.mime, "text/txt");
        assert_eq!(frame.filename, "Lyrics.lrc");
        assert_eq!(frame.desc, "Lyrics");
        assert_eq!(frame.data, b"\x16\x16\x16\x16\x16\x16")
    }

    #[test]
    fn render_apic() {
        let frame = AttachedPictureFrame {
            encoding: Encoding::Latin1,
            mime: String::from("image/png"),
            pic_type: PictureType::FrontCover,
            desc: String::from("Geogaddi_Cover.png"),
            picture: b"\x16\x16\x16\x16\x16\x16".to_vec(),
        };

        assert_render!(frame, APIC_DATA);
    }

    #[test]
    fn render_geob() {
        let frame = GeneralObjectFrame {
            encoding: Encoding::Utf16,
            mime: String::from("text/txt"),
            filename: String::from("Lyrics.lrc"),
            desc: String::from("Lyrics"),
            data: b"\x16\x16\x16\x16\x16\x16".to_vec(),
        };

        assert_render!(frame, GEOB_DATA);
    }
}
