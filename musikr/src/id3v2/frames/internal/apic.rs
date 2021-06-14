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
    pic_data: Vec<u8>,
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
            pic_data: Vec::new(),
        }
    }

    pub fn mime(&self) -> &String {
        &self.mime
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.pic_data
    }

    pub fn pic_type(&self) -> &Type {
        &self.pic_type
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

    fn parse(&mut self, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < self.encoding.nul_size() + 4 {
            return Err(ParseError::NotEnoughData);
        }

        let mime = string::get_terminated_string(Encoding::Utf8, &data[1..]);
        self.mime = mime.string;

        // image/ is implied when there is no mime type.
        if self.mime.is_empty() {
            self.mime = "image/".to_string()
        }

        let mut pos = 1 + mime.size;

        self.pic_type = Type::new(data[pos]);
        pos += 1;

        let desc = string::get_terminated_string(self.encoding, &data[pos..]);
        self.desc = desc.string;
        pos += desc.size;

        self.pic_data = data[pos..].to_vec();

        Ok(())
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
