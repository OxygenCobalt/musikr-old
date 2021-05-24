use crate::id3::frame::ID3Frame;
use crate::id3::frame::FrameHeader;
use crate::id3::frame::string;
use crate::id3::frame::string::ID3Encoding;

pub struct APICFrame<'a> {
    header: FrameHeader,
    pub encoding: ID3Encoding,
    pub mime: String,
    pub desc: String,
    pub pic_type: u8,
    pub pic_data: &'a [u8]
}

impl <'a> ID3Frame for APICFrame<'a> {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }

    fn format(&self) -> String {
        return format![
            "{}: {} {} [{:x?}]", 
            self.header.code, self.mime, self.format_desc(), self.pic_type
        ];
    }
}

impl <'a> APICFrame<'a> {
    fn format_desc(&self) -> String {
        return if self.desc == "" {
            String::new()
        } else {
            format!["\"{}\"", self.desc]
        }
    }
}


impl <'a> APICFrame<'a> {
    pub fn from(header: FrameHeader, data: &[u8]) -> APICFrame {
        // TODO: Create an `at` variable that keeps the current position

        let encoding = string::get_encoding(data[0]);
        let (type_index, mime) = match string::get_nulstring(&ID3Encoding::UTF8, &data[1..]) {
            Some(mime) => (mime.len() + 2, mime),

            // If theres no mime type, "image/" is implied
            None => (2, String::from("image/"))
        };

        let pic_type = data[type_index];

        // Each index is determined by the last one.
        let desc_index = type_index + 1;

        let (pic_index, desc) = match string::get_nulstring(&encoding, &data[desc_index..]) {
            Some(desc) => (desc_index + desc.len() + 1, desc),
            None => (desc_index + 1, String::new())
        };

        let pic_data = &data[pic_index..];

        return APICFrame {
            header, encoding, mime, desc, pic_type, pic_data
        };
    }
}
