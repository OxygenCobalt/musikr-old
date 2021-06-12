use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::time::{Timestamp, TimestampFormat};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader, ParseError};
use crate::raw;
use std::fmt::{self, Display, Formatter};

pub struct UnsyncLyricsFrame {
    header: FrameHeader,
    encoding: Encoding,
    lang: String,
    desc: String,
    lyrics: String,
}

impl UnsyncLyricsFrame {
    pub fn new(header: FrameHeader) -> Self {
        UnsyncLyricsFrame {
            header,
            encoding: Encoding::default(),
            lang: String::new(),
            desc: String::new(),
            lyrics: String::new(),
        }
    }

    pub fn lang(&self) -> &String {
        &self.lang
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn lyrics(&self) -> &String {
        &self.lyrics
    }
}

impl Frame for UnsyncLyricsFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn flags(&self) -> &FrameFlags {
        &self.header.flags
    }

    fn key(&self) -> String {
        format!["{}:{}:{}", self.id(), self.desc, self.lang]
    }

    fn parse(&mut self, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < self.encoding.nul_size() + 5 {
            return Err(ParseError::NotEnoughData);
        }

        self.lang = string::get_string(Encoding::Utf8, &data[1..3]);

        let desc = string::get_terminated_string(self.encoding, &data[4..]);
        self.desc = desc.string;

        let text_pos = 4 + desc.size;
        self.lyrics = string::get_string(self.encoding, &data[text_pos..]);

        Ok(())
    }
}

impl Display for UnsyncLyricsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if !self.desc.is_empty() {
            write![f, "\n{}:", self.desc]?;
        }

        write![f, "\n{}", self.lyrics]
    }
}

pub struct SyncedLyricsFrame {
    header: FrameHeader,
    encoding: Encoding,
    lang: String,
    time_format: TimestampFormat,
    content_type: SyncedContentType,
    desc: String,
    lyrics: Vec<SyncedText>,
}

impl SyncedLyricsFrame {
    pub fn new(header: FrameHeader) -> Self {
        SyncedLyricsFrame {
            header,
            encoding: Encoding::default(),
            lang: String::new(),
            time_format: TimestampFormat::default(),
            content_type: SyncedContentType::default(),
            desc: String::new(),
            lyrics: Vec::new(),
        }
    }

    pub fn lang(&self) -> &String {
        &self.lang
    }

    pub fn time_format(&self) -> TimestampFormat {
        self.time_format
    }

    pub fn content_type(&self) -> SyncedContentType {
        self.content_type
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn lyrics(&self) -> &Vec<SyncedText> {
        &self.lyrics
    }
}

impl Frame for SyncedLyricsFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn flags(&self) -> &FrameFlags {
        &self.header.flags
    }

    fn key(&self) -> String {
        format!["{}:{}:{}", self.id(), self.desc, self.lang]
    }

    fn parse(&mut self, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < self.encoding.nul_size() + 6 {
            return Err(ParseError::NotEnoughData);
        }

        self.lang = String::from_utf8_lossy(&data[1..4]).to_string();
        self.time_format = TimestampFormat::new(data[4]);
        self.content_type = SyncedContentType::new(data[5]);
        let desc = string::get_terminated_string(self.encoding, &data[6..]);
        self.desc = desc.string;

        // For UTF-16 Synced Lyrics frames, a tagger might only write the BOM to the description
        // and nowhere else. If thats the case, we will subsitute the generic Utf16 encoding for
        // the implicit encoding if there is no bom in each lyric.

        let implicit_encoding = match self.encoding {
            Encoding::Utf16Bom => {
                let bom = raw::to_u16(&data[6..8]);

                match bom {
                    0xFFFE => Encoding::Utf16Le,
                    0xFEFF => Encoding::Utf16Be,
                    _ => self.encoding,
                }
            }

            _ => self.encoding,
        };

        let mut pos = desc.size + 6;

        while pos < data.len() {
            let bom = raw::to_u16(&data[pos..pos + 2]);

            // If the lyric does not have a BOM, use the implicit encoding we got earlier.
            let enc = if bom != 0xFEFF && bom != 0xFFFE {
                implicit_encoding
            } else {
                self.encoding
            };

            let text = string::get_terminated_string(enc, &data[pos..]);
            pos += text.size;
            let timestamp = self
                .time_format
                .make_timestamp(raw::to_u32(&data[pos..pos + 4]));
            pos += 4;

            self.lyrics.push(SyncedText {
                text: text.string,
                timestamp,
            })
        }

        Ok(())
    }
}

impl Display for SyncedLyricsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Append a brief header if we have a description, otherwise we omit the content type
        // altogether since it only really works in conjunction with a description
        if !self.desc.is_empty() {
            write![f, "\n\"{}\" [{:?}]:", self.desc, self.content_type]?;
        }

        for lyric in &self.lyrics {
            write![f, "\n{}", lyric]?;
        }

        Ok(())
    }
}

byte_enum! {
    pub enum SyncedContentType {
        Other = 0x00,
        Lyrics = 0x01,
        TextTranscription = 0x02,
        Movement = 0x03,
        Events = 0x04,
        Chord = 0x05,
        Trivia = 0x06,
        WebpageUrls = 0x07,
        ImageUrls = 0x08,
    }
}

impl Default for SyncedContentType {
    fn default() -> Self {
        SyncedContentType::Other
    }
}

pub struct SyncedText {
    pub text: String,
    pub timestamp: Timestamp,
}

impl Display for SyncedText {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Since we are formatting this already, strip any trailing newlines from the lyrics
        // using a somewhat clunky if block.
        let text = if self.text.starts_with('\n') {
            self.text
                .strip_prefix("\r\n")
                .or_else(|| self.text.strip_prefix("\n"))
                .unwrap_or(&self.text)
        } else if self.text.ends_with('\n') {
            self.text
                .strip_suffix("\r\n")
                .or_else(|| self.text.strip_suffix("\n"))
                .unwrap_or(&self.text)
        } else {
            &self.text
        };

        // Don't include the timestamp, as formatting time is beyond the scope of libmusikr
        write![f, "{}", text]
    }
}
