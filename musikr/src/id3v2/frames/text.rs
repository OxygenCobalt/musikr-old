use crate::core::io::BufStream;
use crate::id3v2::frames::{encoding, Frame, FrameId};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use log::info;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct TextFrame {
    frame_id: FrameId,
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
            frame_id,
            encoding: Encoding::default(),
            text: Vec::new(),
        }
    }

    pub(crate) fn parse(frame_id: FrameId, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let text = parse_text(encoding, stream);

        Ok(Self {
            frame_id,
            encoding,
            text,
        })
    }

    pub(crate) fn is_text(frame_id: FrameId) -> bool {
        // Apple's WFED (Podcast URL), MVNM (Movement Name), MVIN (Movement Number),
        // and GRP1 (Grouping) frames are all actually text frames
        frame_id.starts_with(b'T')
            || matches!(frame_id.inner(), b"WFED" | b"MVNM" | b"MVIN" | b"GRP1")
    }
}

impl Frame for TextFrame {
    fn id(&self) -> FrameId {
        self.frame_id
    }

    fn key(&self) -> String {
        self.id().to_string()
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

#[macro_export]
macro_rules! text_frame {
    ($id:expr; $($text:expr),+ $(,)?) => {
        crate::text_frame!($id, Encoding::default(), $text);
    };
    ($id:expr, $enc:expr, $($text:expr),+ $(,)?) => {
        {
            let mut frame = crate::id3v2::frames::TextFrame::new(crate::id3v2::frames::FrameId::new($id));
            frame.encoding = $enc;
            frame.text = vec![$(String::from($text),)*];
            frame
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserTextFrame {
    pub encoding: Encoding,
    pub desc: String,
    pub text: Vec<String>,
}

impl UserTextFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;

        let desc = string::read_terminated(encoding, stream);
        let text = parse_text(encoding, stream);

        Ok(Self {
            encoding,
            desc,
            text,
        })
    }
}

impl Frame for UserTextFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"TXXX")
    }

    fn key(&self) -> String {
        format!["TXXX:{}", self.desc]
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
            encoding: Encoding::default(),
            desc: String::new(),
            text: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreditsFrame {
    frame_id: FrameId,
    pub encoding: Encoding,
    pub people: BTreeMap<String, String>,
}

impl CreditsFrame {
    pub fn new_tipl() -> Self {
        Self {
            frame_id: FrameId::new(b"TIPL"),
            encoding: Encoding::default(),
            people: BTreeMap::new(),
        }
    }

    pub fn new_tmcl() -> Self {
        Self {
            frame_id: FrameId::new(b"TMCL"),
            encoding: Encoding::default(),
            people: BTreeMap::new(),
        }
    }

    pub(crate) fn parse(frame_id: FrameId, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let mut text = parse_text(encoding, stream);

        if text.len() % 2 != 0 {
            // The spec says that IPLS/TIPL/TMCL must contain an even number of entries.
            // If this frame does have an incomplete pair, we just pop it off and move on.
            info!("found an uneven amount of entries in {}, truncating", frame_id);

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
            frame_id,
            encoding,
            people,
        })
    }

    pub fn is_involved_people(&self) -> bool {
        self.id() == b"IPLS" || self.id() == b"TIPL"
    }

    pub fn is_musician_credits(&self) -> bool {
        self.id() == b"TMCL"
    }
}

impl Frame for CreditsFrame {
    fn id(&self) -> FrameId {
        self.frame_id
    }

    fn key(&self) -> String {
        // CreditsFrame uses the ID3v2.4 frames as it's API surface, only collapsing
        // into the version-specific variants when written. This is to prevent IPLS and
        // TIPL from co-existing in the same tag.
        if self.is_involved_people() {
            String::from("TIPL")
        } else {
            String::from("TMCL")
        }
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

#[macro_export]
macro_rules! tipl_frame {
    ($($role:expr => $people:expr),+ $(,)?) => {
        tipl_frame!(crate::id3v2::Encoding::default(), $($role, $people)*)
    };
    ($enc:expr, $($role:expr => $people:expr),+ $(,)?) => {
        {
            let mut frame = CreditsFrame::new_tipl();
            $(frame.people.insert(String::from($role), String::from($people));)*
            frame
        }
    }
}

#[macro_export]
macro_rules! tmcl_frame {
    ($($role:expr => $people:expr),+ $(,)?) => {
        tmcl_frame!(crate::id3v2::Encoding::default(), $($role => $people)*)
    };
    ($enc:expr, $($role:expr => $people:expr),+ $(,)?) => {
        {
            let mut frame = CreditsFrame::new_tmcl();
            frame.encoding = $enc;
            $(frame.people.insert(String::from($role), String::from($people));)*
            frame
        }
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

    const TEXT_STR: &str = "I Swallowed Hard, Like I Understood";

    const TIT2_DATA: &[u8] = b"TIT2\x00\x00\x00\x49\x00\x00\
                               \x01\
                               \xFF\xFE\x49\x00\x20\x00\x53\x00\x77\x00\x61\x00\x6c\x00\x6c\x00\
                               \x6f\x00\x77\x00\x65\x00\x64\x00\x20\x00\x48\x00\x61\x00\x72\x00\
                               \x64\x00\x2c\x00\x20\x00\x4c\x00\x69\x00\x6b\x00\x65\x00\x20\x00\
                               \x49\x00\x20\x00\x55\x00\x6e\x00\x64\x00\x65\x00\x72\x00\x73\x00\
                               \x74\x00\x6f\x00\x6f\x00\x64\x00";

    const TXXX_DATA: &[u8] = b"TXXX\x00\x00\x00\x23\x00\x00\
                               \x00\
                               replaygain_track_gain\0\
                               -7.429688 dB";

    const TMCL_DATA: &[u8] = b"TMCL\x00\x00\x00\x2B\x00\x00\
                               \x00\
                               Bassist\0\
                               John Smith\0\
                               Violinist\0\
                               Vanessa Evans";

    const MULTI_TEXT_DATA: &[u8] = b"Post-Rock\0\
                                     Ambient\0\
                                     Electronica";

    #[test]
    fn parse_text_frame() {
        crate::make_frame!(TextFrame, TIT2_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Utf16);
        assert_eq!(frame.text[0], TEXT_STR);
    }

    #[test]
    fn parse_txxx() {
        crate::make_frame!(UserTextFrame, TXXX_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Latin1);
        assert_eq!(frame.desc, "replaygain_track_gain");
        assert_eq!(frame.text[0], "-7.429688 dB");
    }

    #[test]
    fn parse_credits() {
        crate::make_frame!(CreditsFrame, TMCL_DATA, frame);

        assert!(frame.is_musician_credits());
        assert!(!frame.is_involved_people());
        assert_eq!(frame.encoding, Encoding::Latin1);
        assert_eq!(frame.people["Violinist"], "Vanessa Evans");
        assert_eq!(frame.people["Bassist"], "John Smith");
    }

    #[test]
    fn render_text_frame() {
        let frame = text_frame! { b"TIT2", Encoding::Utf16, TEXT_STR };
        crate::assert_render!(frame, TIT2_DATA);
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
        let frame = UserTextFrame {
            encoding: Encoding::Latin1,
            desc: String::from("replaygain_track_gain"),
            text: vec![String::from("-7.429688 dB")],
        };

        crate::assert_render!(frame, TXXX_DATA);
    }

    #[test]
    fn render_credits() {
        let frame = tmcl_frame! {
            Encoding::Latin1,
            "Violinist" => "Vanessa Evans",
            "Bassist" => "John Smith"
        };

        crate::assert_render!(frame, TMCL_DATA);
    }
}
