use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::ParseError;
use std::fmt::{self, Display, Formatter};

pub struct AttatchedPictureFrame {
    header: FrameHeader,
    encoding: Encoding,
    mime: String,
    desc: String,
    pic_type: Type,
    picture: Vec<u8>,
}

impl AttatchedPictureFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("APIC", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        AttatchedPictureFrame {
            header,
            encoding: Encoding::default(),
            mime: String::new(),
            desc: String::new(),
            pic_type: Type::default(),
            picture: Vec::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
        let encoding = Encoding::parse(data)?;

        if data.len() < encoding.nul_size() + 4 {
            // Must be at least 1 encoding byte, 2 empty terminated strings, 1 type byte,
            // and at least 1 picture byte.
            return Err(ParseError::NotEnoughData);
        }

        let mut mime = string::get_terminated_string(Encoding::Latin1, &data[1..]);
        let mut pos = 1 + mime.size;

        // image/ is implied when there is no mime type.
        if mime.string.is_empty() {
            mime.string.push_str("image/");
        }

        let pic_type = Type::new(data[pos]);
        pos += 1;

        let desc = string::get_terminated_string(encoding, &data[pos..]);
        pos += desc.size;

        let picture = data[pos..].to_vec();

        Ok(AttatchedPictureFrame {
            header,
            encoding,
            mime: mime.string,
            desc: desc.string,
            pic_type,
            picture
        })
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn mime(&self) -> &String {
        &self.mime
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn pic_type(&self) -> &Type {
        &self.pic_type
    }

    pub fn picture(&self) -> &Vec<u8> {
        &self.picture
    }
}

impl Frame for AttatchedPictureFrame {
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        // *Technically* the spec says that there can only be one FileIcon and OtherFileIcon
        // APIC frame per tag, but pretty much no tagger enforces this.
        format!["{}:{}", self.id(), self.desc]
    }
}

impl Display for AttatchedPictureFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{} ", self.mime]?;

        if !self.desc.is_empty() {
            write![f, "\"{}\" ", self.desc]?;
        }

        write![f, "[{:?}]", self.pic_type]
    }
}

impl Default for AttatchedPictureFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}

byte_enum! {
    pub enum Type {
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
    }
}

impl Default for Type {
    fn default() -> Self {
        Type::Other
    }
}

pub struct GeneralObjectFrame {
    header: FrameHeader,
    encoding: Encoding,
    mime: String,
    filename: String,
    desc: String,
    data: Vec<u8>,
}

impl GeneralObjectFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("GEOB", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        GeneralObjectFrame {
            header,
            encoding: Encoding::default(),
            mime: String::new(),
            filename: String::new(),
            desc: String::new(),
            data: Vec::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
        let encoding = Encoding::parse(data)?;

        if data.len() < (encoding.nul_size() * 2) + 3 {
            // Must be at least one encoding byte, three empty terminated strings, and
            // one byte of data.
            return Err(ParseError::NotEnoughData);
        }

        let mime = string::get_terminated_string(Encoding::Latin1, &data[1..]);
        let mut pos = mime.size + 1;

        let filename = string::get_terminated_string(encoding, &data[pos..]);
        pos += filename.size;

        let desc = string::get_terminated_string(encoding, &data[pos..]);
        pos += desc.size;

        let data = data[pos..].to_vec();

        Ok(GeneralObjectFrame {
            header,
            encoding,
            mime: mime.string,
            filename: filename.string,
            desc: desc.string,
            data
        })
    }

    fn encoding(&self) -> Encoding {
        self.encoding
    }

    fn mime(&self) -> &String {
        &self.mime
    }

    fn filename(&self) -> &String {
        &self.filename
    }

    fn desc(&self) -> &String {
        &self.desc
    }

    fn data(&self) -> &Vec<u8> {
        &self.data
    }
}

impl Frame for GeneralObjectFrame {
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        format!["{}:{}", self.id(), self.desc]
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

impl Default for GeneralObjectFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_apic() {
        let data = b"\x00\
                     \x69\x6D\x61\x67\x65\x2F\x70\x6E\x67\0\
                     \x03\
                     \x47\x65\x6F\x67\x61\x64\x64\x69\x5F\x43\x6F\x76\x65\x72\x2E\x70\x6E\x67\0\
                     \x16\x16\x16\x16\x16";

        let frame = AttatchedPictureFrame::parse(FrameHeader::new("APIC"), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Latin1);
        assert_eq!(frame.mime(), "image/png");
        assert_eq!(frame.desc(), "Geogaddi_Cover.png");
        assert_eq!(frame.picture(), b"\x16\x16\x16\x16\x16");
    }

    #[test]
    fn parse_geob() {
        let data = b"\x01\
                     text/txt\0\
                     \xFF\xFE\x4c\x00\x79\x00\x72\x00\x69\x00\x63\x00\x73\x00\x2e\x00\x6c\x00\x72\x00\x63\x00\0\0\
                     \xFF\xFE\x4c\x00\x79\x00\x72\x00\x69\x00\x63\x00\x73\x00\0\0\
                     \x16\x16\x16\x16\x16";

        let frame = GeneralObjectFrame::parse(FrameHeader::new("GEOB"), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf16);
        assert_eq!(frame.mime(), "text/txt");
        assert_eq!(frame.filename(), "Lyrics.lrc");
        assert_eq!(frame.desc(), "Lyrics");
        assert_eq!(frame.data(), b"\x16\x16\x16\x16\x16")
    }
}
