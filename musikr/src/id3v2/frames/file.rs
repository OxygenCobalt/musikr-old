use crate::id3v2::frames::{encoding, Frame, FrameConfig, FrameHeader};
use crate::id3v2::{ParseError, ParseResult, TagHeader, Token};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

pub struct AttachedPictureFrame {
    header: FrameHeader,
    encoding: Encoding,
    mime: String,
    desc: String,
    pic_type: PictureType,
    picture: Vec<u8>,
}

impl AttachedPictureFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameConfig) -> Self {
        Self::with_header(FrameHeader::with_flags("APIC", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        AttachedPictureFrame {
            header,
            encoding: Encoding::default(),
            mime: String::new(),
            desc: String::new(),
            pic_type: PictureType::default(),
            picture: Vec::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> ParseResult<Self> {
        let encoding = encoding::get(data)?;

        if data.len() < encoding.nul_size() + 4 {
            // Must be at least 1 encoding byte, 2 empty terminated strings, 1 type byte,
            // and at least 1 picture byte.
            return Err(ParseError::NotEnoughData);
        }

        let mut mime = string::get_terminated(Encoding::Latin1, &data[1..]);

        // image/ is implied when there is no mime type.
        if mime.string.is_empty() {
            mime.string.push_str("image/");
        }

        let mut pos = 1 + mime.size;

        let pic_type = PictureType::new(data[pos]);
        pos += 1;

        let desc = string::get_terminated(encoding, &data[pos..]);
        pos += desc.size;

        let picture = data[pos..].to_vec();

        Ok(AttachedPictureFrame {
            header,
            encoding,
            mime: mime.string,
            desc: desc.string,
            pic_type,
            picture,
        })
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn mime(&self) -> &String {
        &self.mime
    }

    pub fn pic_type(&self) -> PictureType {
        self.pic_type
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn picture(&self) -> &Vec<u8> {
        &self.picture
    }

    pub fn encoding_mut(&mut self) -> &mut Encoding {
        &mut self.encoding
    }

    pub fn mime_mut(&mut self) -> &mut String {
        &mut self.mime
    }

    pub fn pic_type_mut(&mut self) -> &mut PictureType {
        &mut self.pic_type
    }

    pub fn desc_mut(&mut self) -> &mut String {
        &mut self.desc
    }

    pub fn picture_mut(&mut self) -> &mut Vec<u8> {
        &mut self.picture
    }
}

impl Frame for AttachedPictureFrame {
    fn key(&self) -> String {
        // *Technically* the spec says that there can only be one FileIcon and OtherFileIcon
        // APIC frame per tag, but pretty much no tagger enforces this.
        format!["{}:{}", self.id(), self.desc]
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.picture.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.major());
        result.push(encoding::render(self.encoding));

        result.extend(string::render_terminated(Encoding::Latin1, &self.mime));
        result.push(self.pic_type as u8);
        result.extend(string::render_terminated(encoding, &self.desc));
        result.extend(self.picture.clone());

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

impl Default for AttachedPictureFrame {
    fn default() -> Self {
        Self::with_flags(FrameConfig::default())
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
    }
}

impl Default for PictureType {
    fn default() -> Self {
        PictureType::Other
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

    pub fn with_flags(flags: FrameConfig) -> Self {
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

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> ParseResult<Self> {
        let encoding = encoding::get(data)?;

        if data.len() < (encoding.nul_size() * 2) + 3 {
            // Must be at least one encoding byte, three empty terminated strings, and
            // one byte of data.
            return Err(ParseError::NotEnoughData);
        }

        let mime = string::get_terminated(Encoding::Latin1, &data[1..]);
        let mut pos = mime.size + 1;

        let filename = string::get_terminated(encoding, &data[pos..]);
        pos += filename.size;

        let desc = string::get_terminated(encoding, &data[pos..]);
        pos += desc.size;

        let data = data[pos..].to_vec();

        Ok(GeneralObjectFrame {
            header,
            encoding,
            mime: mime.string,
            filename: filename.string,
            desc: desc.string,
            data,
        })
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn mime(&self) -> &String {
        &self.mime
    }

    pub fn filename(&self) -> &String {
        &self.filename
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn encoding_mut(&mut self) -> &mut Encoding {
        &mut self.encoding
    }

    pub fn mime_mut(&mut self) -> &mut String {
        &mut self.mime
    }

    pub fn filename_mut(&mut self) -> &mut String {
        &mut self.filename
    }

    pub fn desc_mut(&mut self) -> &mut String {
        &mut self.desc
    }

    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }
}

impl Frame for GeneralObjectFrame {
    fn key(&self) -> String {
        format!["{}:{}", self.id(), self.desc]
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.major());
        result.push(encoding::render(self.encoding));

        result.extend(string::render_terminated(Encoding::Latin1, &self.mime));
        result.extend(string::render_terminated(encoding, &self.filename));
        result.extend(string::render_terminated(encoding, &self.desc));
        result.extend(self.data.clone());

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

impl Default for GeneralObjectFrame {
    fn default() -> Self {
        Self::with_flags(FrameConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const APIC_DATA: &[u8] = b"\x00\
                               image/png\0\
                               \x03\
                               Geogaddi_Cover.png\0\
                               \x16\x16\x16\x16\x16\x16";

    const GEOB_DATA: &[u8] = b"\x01\
                               text/txt\0\
                               \xFF\xFE\x4c\x00\x79\x00\x72\x00\x69\x00\x63\x00\x73\x00\x2e\x00\x6c\x00\x72\x00\x63\x00\0\0\
                               \xFF\xFE\x4c\x00\x79\x00\x72\x00\x69\x00\x63\x00\x73\x00\0\0\
                               \x16\x16\x16\x16\x16\x16";

    #[test]
    fn parse_apic() {
        let frame = AttachedPictureFrame::parse(FrameHeader::new("APIC"), APIC_DATA).unwrap();

        assert_eq!(frame.encoding(), Encoding::Latin1);
        assert_eq!(frame.mime(), "image/png");
        assert_eq!(frame.pic_type(), PictureType::FrontCover);
        assert_eq!(frame.desc(), "Geogaddi_Cover.png");
        assert_eq!(frame.picture(), b"\x16\x16\x16\x16\x16\x16");
    }

    #[test]
    fn parse_geob() {
        let frame = GeneralObjectFrame::parse(FrameHeader::new("GEOB"), GEOB_DATA).unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf16);
        assert_eq!(frame.mime(), "text/txt");
        assert_eq!(frame.filename(), "Lyrics.lrc");
        assert_eq!(frame.desc(), "Lyrics");
        assert_eq!(frame.data(), b"\x16\x16\x16\x16\x16\x16")
    }

    #[test]
    fn render_apic() {
        let mut frame = AttachedPictureFrame::new();

        *frame.encoding_mut() = Encoding::Latin1;
        frame.mime_mut().push_str("image/png");
        *frame.pic_type_mut() = PictureType::FrontCover;
        frame.desc_mut().push_str("Geogaddi_Cover.png");
        *frame.picture_mut() = vec![0x16, 0x16, 0x16, 0x16, 0x16, 0x16];

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), APIC_DATA);
    }

    #[test]
    fn render_geob() {
        let mut frame = GeneralObjectFrame::new();

        *frame.encoding_mut() = Encoding::Utf16;
        frame.mime_mut().push_str("text/txt");
        frame.filename_mut().push_str("Lyrics.lrc");
        frame.desc_mut().push_str("Lyrics");
        *frame.data_mut() = vec![0x16, 0x16, 0x16, 0x16, 0x16, 0x16];

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), GEOB_DATA);
    }
}
