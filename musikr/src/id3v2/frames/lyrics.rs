use crate::core::io::BufStream;
use crate::id3v2::frames::lang::Language;
use crate::id3v2::frames::time::TimestampFormat;
use crate::id3v2::frames::{encoding, Frame, FrameHeader, Token};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

pub struct UnsyncLyricsFrame {
    header: FrameHeader,
    pub encoding: Encoding,
    pub lang: Language,
    pub desc: String,
    pub lyrics: String,
}

impl UnsyncLyricsFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let lang = Language::parse(stream)?;
        let desc = string::read_terminated(encoding, stream);
        let lyrics = string::read(encoding, stream);

        Ok(Self {
            header,
            encoding,
            lang,
            desc,
            lyrics,
        })
    }
}

impl Frame for UnsyncLyricsFrame {
    fn key(&self) -> String {
        format!["USLT:{}:{}", self.desc, self.lang]
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.lyrics.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.major());
        result.push(encoding::render(self.encoding));

        result.extend(&self.lang);

        result.extend(string::render_terminated(encoding, &self.desc));
        result.extend(string::render(encoding, &self.lyrics));

        result
    }
}

impl Display for UnsyncLyricsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if !self.desc.is_empty() {
            writeln![f, "{}:", self.desc]?;
        }

        write![f, "{}", self.lyrics]?;

        Ok(())
    }
}

impl Default for UnsyncLyricsFrame {
    fn default() -> Self {
        Self {
            header: FrameHeader::new(b"USLT"),
            encoding: Encoding::default(),
            lang: Language::default(),
            desc: String::new(),
            lyrics: String::new(),
        }
    }
}

pub struct SyncedLyricsFrame {
    header: FrameHeader,
    pub encoding: Encoding,
    pub lang: Language,
    pub format: TimestampFormat,
    pub content_type: SyncedContentType,
    pub desc: String,
    pub lyrics: Vec<SyncedText>,
}

impl SyncedLyricsFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;

        let lang = Language::parse(stream)?;
        let format = TimestampFormat::parse(stream.read_u8()?);
        let content_type = SyncedContentType::parse(stream.read_u8()?);

        // For UTF-16 Synced Lyrics frames, a tagger might only write the BOM to the description
        // and nowhere else. If thats the case, we will subsitute the generic Utf16 encoding for
        // the implicit encoding if there is no bom in each lyric.

        let implicit_enc = match encoding {
            Encoding::Utf16 => {
                let bom = stream.peek(0..2);

                match bom {
                    Ok([0xFF, 0xFE]) => Encoding::Utf16Le,
                    Ok([0xFE, 0xFF]) => Encoding::Utf16Be,
                    Ok(_) | Err(_) => encoding,
                }
            }

            _ => encoding,
        };

        let desc = string::read_terminated(encoding, stream);

        let mut lyrics: Vec<SyncedText> = Vec::new();

        while !stream.is_empty() {
            // If the lyric does not have a BOM, use the implicit encoding we got earlier.
            let bom = stream.peek(0..2)?;

            let enc = if bom != [0xFF, 0xFE] && bom != [0xFE, 0xFF] {
                implicit_enc
            } else {
                encoding
            };

            let text = string::read_terminated(enc, stream);
            let time = stream.read_u32()?;

            lyrics.push(SyncedText { text, time })
        }

        Ok(SyncedLyricsFrame {
            header,
            encoding,
            lang,
            format,
            content_type,
            desc,
            lyrics,
        })
    }
}

impl Frame for SyncedLyricsFrame {
    fn key(&self) -> String {
        format!["SYLT:{}:{}", self.desc, self.lang]
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.lyrics.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.major());
        result.push(encoding::render(self.encoding));

        result.extend(&self.lang);

        result.push(self.format as u8);
        result.push(self.content_type as u8);

        result.extend(string::render_terminated(encoding, &self.desc));

        for lyric in &self.lyrics {
            result.extend(string::render_terminated(encoding, &lyric.text));
            result.extend(lyric.time.to_be_bytes());
        }

        result
    }
}

impl Display for SyncedLyricsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Append a brief header if we have a description, otherwise we omit the content type
        // altogether since it only really works in conjunction with a description
        if !self.desc.is_empty() {
            writeln![f, "\"{}\" [{:?}]:", self.desc, self.content_type]?;
        }

        for (i, lyric) in self.lyrics.iter().enumerate() {
            if i < self.lyrics.len() - 1 {
                writeln![f, "{}", lyric]?;
            } else {
                write![f, "{}", lyric]?;
            }
        }

        Ok(())
    }
}

impl Default for SyncedLyricsFrame {
    fn default() -> Self {
        Self {
            header: FrameHeader::new(b"SYLT"),
            encoding: Encoding::default(),
            format: TimestampFormat::default(),
            content_type: SyncedContentType::default(),
            lang: Language::default(),
            desc: String::new(),
            lyrics: Vec::new(),
        }
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
    };
    SyncedContentType::Other
}

impl Default for SyncedContentType {
    fn default() -> Self {
        SyncedContentType::Lyrics
    }
}

pub struct SyncedText {
    pub text: String,
    pub time: u32,
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

        // Don't include the time, as formatting time is beyond the scope of this library
        write![f, "{}", text]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const USLT_DATA: &[u8] = b"\x00\
                                eng\
                                Description\0\
                                Jumped in the river, what did I see?\n\
                                Black eyed angels swam with me\n";

    const SYLT_DATA: &[u8] = b"\x03\
                                eng\
                                \x02\x01\
                                Description\0\
                                You don't remember, you don't remember\n\0\
                                \x00\x02\x78\xD0\
                                Why don't you remember my name?\n\0\
                                \x00\x02\x88\x70";

    #[test]
    fn parse_uslt() {
        let frame =
            UnsyncLyricsFrame::parse(FrameHeader::new(b"USLT"), &mut BufStream::new(USLT_DATA))
                .unwrap();

        assert_eq!(frame.encoding, Encoding::Latin1);
        assert_eq!(frame.lang.as_str(), "eng");
        assert_eq!(frame.desc, "Description");
        assert_eq!(
            frame.lyrics,
            "Jumped in the river, what did I see?\n\
            Black eyed angels swam with me\n"
        )
    }

    #[test]
    fn parse_sylt() {
        let frame =
            SyncedLyricsFrame::parse(FrameHeader::new(b"SYLT"), &mut BufStream::new(SYLT_DATA))
                .unwrap();

        assert_eq!(frame.encoding, Encoding::Utf8);
        assert_eq!(frame.lang.as_str(), "eng");
        assert_eq!(frame.format, TimestampFormat::Millis);
        assert_eq!(frame.content_type, SyncedContentType::Lyrics);
        assert_eq!(frame.desc, "Description");

        assert_eq!(frame.lyrics[0].time, 162_000);
        assert_eq!(frame.lyrics[0].text, "You don't remember, you don't remember\n");
        assert_eq!(frame.lyrics[1].time, 166_000);
        assert_eq!(frame.lyrics[1].text, "Why don't you remember my name?\n");
    }

    #[test]
    fn parse_bomless_sylt() {
        let data = b"\x01\
                     eng\
                     \x02\x01\
                     \xFF\xFE\x44\x00\x65\x00\x73\x00\x63\x00\x72\x00\x69\x00\x70\x00\
                     \x74\x00\x69\x00\x6f\x00\x6e\x00\0\0\
                     \x59\x00\x6f\x00\x75\x00\x20\x00\x64\x00\x6f\x00\x6e\x00\
                     \x27\x00\x74\x00\x20\x00\x72\x00\x65\x00\x6d\x00\x65\x00\x6d\x00\
                     \x62\x00\x65\x00\x72\x00\x2c\x00\x20\x00\x79\x00\x6f\x00\x75\x00\
                     \x20\x00\x64\x00\x6f\x00\x6e\x00\x27\x00\x74\x00\x20\x00\x72\x00\
                     \x65\x00\x6d\x00\x65\x00\x6d\x00\x62\x00\x65\x00\x72\x00\x0a\x00\0\0\
                     \x00\x02\x78\xD0\
                     \x57\x00\x68\x00\x79\x00\x20\x00\x64\x00\x6f\x00\x6e\x00\
                     \x27\x00\x74\x00\x20\x00\x79\x00\x6f\x00\x75\x00\x20\x00\x72\x00\
                     \x65\x00\x6d\x00\x65\x00\x6d\x00\x62\x00\x65\x00\x72\x00\x20\x00\
                     \x6d\x00\x79\x00\x20\x00\x6e\x00\x61\x00\x6d\x00\x65\x00\x3f\x00\
                     \x0a\x00\0\0\
                     \x00\x02\x88\x70";

        let frame =
            SyncedLyricsFrame::parse(FrameHeader::new(b"SYLT"), &mut BufStream::new(data)).unwrap();

        assert_eq!(frame.encoding, Encoding::Utf16);
        assert_eq!(frame.lang.as_str(), "eng");
        assert_eq!(frame.format, TimestampFormat::Millis);
        assert_eq!(frame.content_type, SyncedContentType::Lyrics);
        assert_eq!(frame.desc, "Description");

        assert_eq!(frame.lyrics[0].time, 162_000);
        assert_eq!(frame.lyrics[0].text, "You don't remember, you don't remember\n");
        assert_eq!(frame.lyrics[1].time, 166_000);
        assert_eq!(frame.lyrics[1].text, "Why don't you remember my name?\n");
    }

    #[test]
    fn render_uslt() {
        let mut frame = UnsyncLyricsFrame::new();

        frame.encoding = Encoding::Latin1;
        frame.lang.set(b"eng").unwrap();
        frame.desc.push_str("Description");
        frame.lyrics.push_str(
            "Jumped in the river, what did I see?\n\
             Black eyed angels swam with me\n",
        );

        assert_eq!(frame.render(&TagHeader::with_version(4)), USLT_DATA);
    }

    #[test]
    fn render_sylt() {
        let mut frame = SyncedLyricsFrame::new();

        frame.encoding = Encoding::Utf8;
        frame.lang.set(b"eng").unwrap();
        frame.format = TimestampFormat::Millis;
        frame.content_type = SyncedContentType::Lyrics;
        frame.desc.push_str("Description");
        frame.lyrics = vec![
            SyncedText {
                text: String::from("You don't remember, you don't remember\n"),
                time: 162_000,
            },
            SyncedText {
                text: String::from("Why don't you remember my name?\n"),
                time: 166_000,
            },
        ];

        assert_eq!(frame.render(&TagHeader::with_version(4)), SYLT_DATA)
    }
}
