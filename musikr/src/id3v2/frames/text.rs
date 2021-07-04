use crate::core::io::BufStream;
use crate::id3v2::frames::{encoding, Frame, FrameHeader, FrameId, Token};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct TextFrame {
    header: FrameHeader,
    pub encoding: Encoding,
    pub text: Vec<String>,
}

impl TextFrame {
    pub fn new(frame_id: FrameId) -> Self {
        // Disallow the text frame derivatives from being implemented to prevent the creation
        // of a malformed frame.
        if !Self::is_text(frame_id) || matches!(frame_id.inner(), b"TIPL" | b"TMCL" | b"TXXX") {
            panic!("Expected a valid text frame ID, found {}", frame_id);
        }

        Self {
            header: FrameHeader::new(frame_id),
            encoding: Encoding::default(),
            text: Vec::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let text = parse_text(encoding, stream);

        Ok(Self {
            header,
            encoding,
            text,
        })
    }

    pub fn is_text(frame_id: FrameId) -> bool {
        // Apple's WFED (Podcast URL), MVNM (Movement Name), MVIN (Movement Number),
        // and GRP1 (Grouping) frames are all actually text frames
        frame_id.starts_with(b'T') || matches!(frame_id.inner(), b"WFED" | b"MVNM" | b"MVIN" | b"GRP1")
    }
}

impl Frame for TextFrame {
    fn key(&self) -> String {
        self.id().to_string()
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.version());
        result.push(encoding::render(self.encoding));

        result.extend(render_text(encoding, &self.text));

        result
    }
}

impl Display for TextFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt_text(&self.text, f)
    }
}

#[derive(Debug, Clone)]
pub struct UserTextFrame {
    header: FrameHeader,
    pub encoding: Encoding,
    pub desc: String,
    pub text: Vec<String>,
}

impl UserTextFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;

        let desc = string::read_terminated(encoding, stream);
        let text = parse_text(encoding, stream);

        Ok(Self {
            header,
            encoding,
            desc,
            text,
        })
    }
}

impl Frame for UserTextFrame {
    fn key(&self) -> String {
        format!["TXXX:{}", self.desc]
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.desc.is_empty() && self.text.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.version());
        result.push(encoding::render(self.encoding));

        // Append the description
        result.extend(string::render_terminated(encoding, &self.desc));

        // Then append the text normally.
        result.extend(render_text(encoding, &self.text));

        result
    }
}

impl Display for UserTextFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt_text(&self.text, f)
    }
}

impl Default for UserTextFrame {
    fn default() -> Self {
        Self {
            header: FrameHeader::new(FrameId::new(b"TXXX")),
            encoding: Encoding::default(),
            desc: String::new(),
            text: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreditsFrame {
    header: FrameHeader,
    pub encoding: Encoding,
    pub people: BTreeMap<String, String>,
}

impl CreditsFrame {
    pub fn new_tipl() -> Self {
        Self {
            header: FrameHeader::new(FrameId::new(b"TIPL")),
            encoding: Encoding::default(),
            people: BTreeMap::new(),
        }
    }

    pub fn new_tmcl() -> Self {
        Self {
            header: FrameHeader::new(FrameId::new(b"TMCL")),
            encoding: Encoding::default(),
            people: BTreeMap::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let mut text = parse_text(encoding, stream);

        if text.len() % 2 != 0 {
            // The spec says that TIPL must contain an even number of entries.
            // If this frame does have an incomplete pair, we just pop it off and move on.
            text.pop();
        }

        // Collect the parsed text into a single people map by role -> person.
        let mut people = BTreeMap::new();
        let mut text = text.into_iter();

        while let Some(role) = text.next() {
            // We eliminated the possibility of an incomplete pair earlier, so we can
            // just unwrap here
            let role_people = text.next().unwrap();

            people.insert(role, role_people);
        }

        Ok(Self {
            header,
            encoding,
            people,
        })
    }

    pub fn is_involved_people(&self) -> bool {
        self.id() == "IPLS" || self.id() == "TMCL"
    }

    pub fn is_musician_credits(&self) -> bool {
        self.id() == "TIPL"
    }
}

impl Frame for CreditsFrame {
    fn key(&self) -> String {
        // CreditsFrame uses the ID3v2.4 frames as it's API surface, only collapsing
        // into the version-specific variants when written. To prevent IPLS and TIPL from
        // coexisting in the same tag, we automatically change the IDs dependencing on
        // the state.
        if self.is_involved_people() {
            String::from("TIPL")
        } else {
            String::from("TMCL")
        }
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.people.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.version());
        result.push(encoding::render(self.encoding));

        // Rendering a CreditsFrame is similar to a TextFrame, but has to be done
        // in pairs since there seems to be no way to zip keys and values into
        // an iterator without having to bring in a dependency.
        for (i, (role, people)) in self.people.iter().enumerate() {
            if i > 0 {
                result.resize(result.len() + encoding.nul_size(), 0);
            }

            result.extend(string::render_terminated(encoding, role));
            result.extend(string::render(encoding, people));
        }

        result
    }
}

impl Display for CreditsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for (i, (role, people)) in self.people.iter().enumerate() {
            if i < self.people.len() - 1 {
                writeln![f, "{}: {}", role, people]?;
            } else {
                write![f, "{}: {}", role, people]?;
            }
        }

        Ok(())
    }
}

fn fmt_text(text: &[String], f: &mut Formatter) -> fmt::Result {
    for (i, string) in text.iter().enumerate() {
        write![f, "{}", string]?;

        if i < text.len() - 1 {
            write![f, ", "]?;
        }
    }

    Ok(())
}

fn parse_text(encoding: Encoding, stream: &mut BufStream) -> Vec<String> {
    // Text frames can contain multiple strings seperated by a NUL terminator, so we have to
    // manually iterate and find each terminated string. If there are none, then the Vec should
    // just contain one string without any issue.
    let mut text = Vec::new();

    while !stream.is_empty() {
        text.push(string::read_terminated(encoding, stream))
    }

    text
}

fn render_text(encoding: Encoding, text: &[String]) -> Vec<u8> {
    let mut result = Vec::new();

    for (i, string) in text.iter().enumerate() {
        // Seperate each string by a NUL except for the last string.
        // For frames with a single string, there will be no NUL terminator.

        if i > 0 {
            result.resize(result.len() + encoding.nul_size(), 0)
        }

        result.extend(string::render(encoding, string));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::tag::Version;

    const TEXT_STR: &str = "I Swallowed Hard, Like I Understood";

    const TEXT_DATA: &[u8] = b"\x01\
                               \xFF\xFE\x49\x00\x20\x00\x53\x00\x77\x00\x61\x00\x6c\x00\x6c\x00\
                               \x6f\x00\x77\x00\x65\x00\x64\x00\x20\x00\x48\x00\x61\x00\x72\x00\
                               \x64\x00\x2c\x00\x20\x00\x4c\x00\x69\x00\x6b\x00\x65\x00\x20\x00\
                               \x49\x00\x20\x00\x55\x00\x6e\x00\x64\x00\x65\x00\x72\x00\x73\x00\
                               \x74\x00\x6f\x00\x6f\x00\x64\x00";

    const TXXX_DATA: &[u8] = b"\x00\
                               replaygain_track_gain\0\
                               -7.429688 dB";

    const TIPL_DATA: &[u8] = b"\x00\
                               Bassist\0\
                               John Smith\0\
                               Violinist\0\
                               Vanessa Evans";

    const MULTI_TEXT_DATA: &[u8] = b"Post-Rock\0\
                                     Ambient\0\
                                     Electronica";

    #[test]
    fn parse_text_frame() {
        let frame = TextFrame::parse(
            FrameHeader::new(FrameId::new(b"TIT2")),
            &mut BufStream::new(TEXT_DATA),
        )
        .unwrap();

        assert_eq!(frame.encoding, Encoding::Utf16);
        assert_eq!(frame.text[0], TEXT_STR);
    }

    #[test]
    fn parse_txxx() {
        let frame = UserTextFrame::parse(
            FrameHeader::new(FrameId::new(b"TXXX")),
            &mut BufStream::new(TXXX_DATA),
        )
        .unwrap();

        assert_eq!(frame.encoding, Encoding::Latin1);
        assert_eq!(frame.desc, "replaygain_track_gain");
        assert_eq!(frame.text[0], "-7.429688 dB");
    }

    #[test]
    fn parse_credits() {
        let frame = CreditsFrame::parse(
            FrameHeader::new(FrameId::new(b"TMCL")),
            &mut BufStream::new(TIPL_DATA),
        )
        .unwrap();

        assert_eq!(frame.encoding, Encoding::Latin1);
        assert_eq!(frame.people["Violinist"], "Vanessa Evans");
        assert_eq!(frame.people["Bassist"], "John Smith");
    }

    #[test]
    fn render_text_frame() {
        let mut frame = TextFrame::new(FrameId::new(b"TIT2"));
        frame.encoding = Encoding::Utf16;
        frame.text.push(String::from(TEXT_STR));

        assert!(!frame.is_empty());
        assert_eq!(
            frame.render(&TagHeader::with_version(Version::V24)),
            TEXT_DATA
        )
    }

    #[test]
    fn render_multi_text() {
        let data = vec![
            "Post-Rock".to_string(),
            "Ambient".to_string(),
            "Electronica".to_string(),
        ];

        assert_eq!(render_text(Encoding::Latin1, &data), MULTI_TEXT_DATA);
    }

    #[test]
    fn render_txxx() {
        let mut frame = UserTextFrame::new();
        frame.encoding = Encoding::Latin1;
        frame.desc.push_str("replaygain_track_gain");
        frame.text.push(String::from("-7.429688 dB"));

        assert!(!frame.is_empty());
        assert_eq!(
            frame.render(&TagHeader::with_version(Version::V24)),
            TXXX_DATA
        );
    }

    #[test]
    fn render_credits() {
        let mut frame = CreditsFrame::new_tmcl();
        frame.encoding = Encoding::Latin1;
        frame
            .people
            .insert("Violinist".to_string(), "Vanessa Evans".to_string());
        frame
            .people
            .insert("Bassist".to_string(), "John Smith".to_string());

        assert!(!frame.is_empty());
        assert_eq!(
            frame.render(&TagHeader::with_version(Version::V24)),
            TIPL_DATA
        );
    }
}
