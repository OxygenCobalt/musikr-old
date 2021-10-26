use crate::core::io::BufStream;
use crate::id3v2::frames::{encoding, Frame, FrameId};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use log::{info, warn};
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
        if !Self::is_id(frame_id) {
            panic!("expected a valid text frame id, found {}", frame_id);
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

    pub fn is_id(frame_id: FrameId) -> bool {
        is_id!(
            // Text
            frame_id, b"TALB", b"TCOM", b"TCON", b"TCOP", b"TENC", b"TEXT", b"TFLT", b"TIT1",
            b"TIT2", b"TIT3", b"TKEY", b"TLAN", b"TMED", b"TOAL", b"TOFN", b"TOLY", b"TOPE",
            b"TOWN", b"TPE1", b"TPE2", b"TPE3", b"TPE4", b"TPUB", b"TRSN", b"TRSO", b"TSRC",
            b"TSSE", b"TRDA", b"TMOO", b"TPRO", b"TSOA", b"TSOP", b"TSOT", b"TSST", b"TSO2",
            b"TSOC", b"TCAT", b"TDES", b"TGID", // Numeric
            b"TLEN", b"TYER", b"TDAT", b"TIME", b"TORY", b"TSIZ", b"TCMP", b"TDLY", b"TBPM",
            // Numeric part
            b"TPOS", b"TRCK", // Timestamps
            b"TDEN", b"TDOR", b"TDRC", b"TDRL", b"TDTG",
            // iTunes WFED [Podcast URL], MVNM [Movement Name], MVIN [Movement Number],
            // and GRP1 [Grouping] are all actually text frames
            b"WFED", b"MVNM", b"MVIN", b"GRP1"
        )
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
        self.text.iter().filter(|text| !text.is_empty()).count() == 0
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
pub struct CreditsFrame {
    frame_id: FrameId,
    pub encoding: Encoding,
    pub people: BTreeMap<String, String>,
}

impl Frame for CreditsFrame {
    fn id(&self) -> FrameId {
        self.frame_id
    }

    fn key(&self) -> String {
        // CreditsFrame uses the ID3v2.4 frames as it's API surface, only collapsing
        // into the version-specific variants when written. This is to prevent IPLS and
        // TIPL from co-existing in the same tag.
        match self.frame_id.inner() {
            b"TIPL" | b"IPLS" => String::from("TIPL"),
            b"TMCL" => String::from("TMCL"),
            _ => unreachable!(),
        }
    }

    fn is_empty(&self) -> bool {
        self.people
            .iter()
            .filter(|(people, role)| !role.is_empty() && !people.is_empty())
            .count()
            == 0
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.version());
        result.push(encoding::render(self.encoding));

        // To prevent lone pairs causing malformed frames, we filter out all
        // role-people pairs that are partially or completely empty.
        let people = self.people.iter().filter(|(role, people)| {
            if role.is_empty() || people.is_empty() {
                warn!("dropping incomplete role-people pair in {}", self.frame_id);
                false
            } else {
                true
            }
        });

        // Rendering a CreditsFrame is similar to a TextFrame, but has to be done
        // in pairs since there seems to be no way to zip keys and values into
        // an iterator without having to bring in a dependency.
        for (i, (role, people)) in people.enumerate() {
            if i > 0 {
                result.resize(result.len() + encoding.nul_size(), 0);
            }

            result.extend(string::render_terminated(encoding, role));
            result.extend(string::render(encoding, people));
        }

        result
    }
}

impl CreditsFrame {
    pub fn new(frame_id: FrameId) -> Self {
        if !Self::is_id(frame_id) {
            panic!("expected a valid credits frame id, found {}", frame_id)
        }

        Self {
            frame_id,
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
            info!(
                "found an uneven amount of entries in {}, truncating",
                frame_id
            );

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

    pub fn is_id(frame_id: FrameId) -> bool {
        is_id!(frame_id, b"IPLS", b"TIPL", b"TMCL")
    }

    pub(crate) fn id_mut(&mut self) -> &mut FrameId {
        &mut self.frame_id
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
        self.desc.is_empty() && self.text.iter().filter(|text| !text.is_empty()).count() == 0
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

fn fmt_text<D: Display>(text: &[D], f: &mut Formatter) -> fmt::Result {
    for (i, string) in text.iter().enumerate() {
        write![f, "{}", string]?;

        if i < text.len() - 1 {
            write![f, ", "]?;
        }
    }

    Ok(())
}

fn parse_text(encoding: Encoding, stream: &mut BufStream) -> Vec<String> {
    // Text frames can contain multiple strings separated by a NUL terminator, so we have to
    // manually iterate and find each terminated string. If there are none, then the Vec should
    // just contain one string without any issue. This technically isnt supported in ID3v2.3, but
    // everyone does it anyway.
    let mut text = Vec::new();

    while !stream.is_empty() {
        let string = string::read_terminated(encoding, stream);

        // Sometimes taggers will pad their text frames with zeroes. To prevent these from being
        // recognized as empty strings, we will only add strings if they have actual content in
        // them.
        if !string.is_empty() {
            text.push(string);
        }
    }

    text
}

fn render_text(encoding: Encoding, text: &[String]) -> Vec<u8> {
    let mut result = Vec::new();

    for (i, string) in text.iter().enumerate() {
        // Separate each string by a NUL except for the last string.
        // For frames with a single string, there will be no NUL terminator.

        if i > 0 {
            result.resize(result.len() + encoding.nul_size(), 0)
        }

        result.extend(string::render(encoding, &string.to_string()));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIT2_DATA: &[u8] = b"TIT2\x00\x00\x00\x49\x00\x00\
                               \x01\
                               \xFF\xFE\x49\x00\x20\x00\x53\x00\x77\x00\x61\x00\x6c\x00\x6c\x00\
                               \x6f\x00\x77\x00\x65\x00\x64\x00\x20\x00\x48\x00\x61\x00\x72\x00\
                               \x64\x00\x2c\x00\x20\x00\x4c\x00\x69\x00\x6b\x00\x65\x00\x20\x00\
                               \x49\x00\x20\x00\x55\x00\x6e\x00\x64\x00\x65\x00\x72\x00\x73\x00\
                               \x74\x00\x6f\x00\x6f\x00\x64\x00";

    const TCON_DATA: &[u8] = b"TCON\x00\x00\x00\x17\x00\x00\
                               \x00\
                               Post-Rock\0\
                               Electronica\0";

    const TMCL_DATA: &[u8] = b"TMCL\x00\x00\x00\x2B\x00\x00\
                               \x00\
                               Bassist\0\
                               John Smith\0\
                               Violinist\0\
                               Vanessa Evans";

    const TXXX_DATA: &[u8] = b"TXXX\x00\x00\x00\x23\x00\x00\
                               \x00\
                               replaygain_track_gain\0\
                               -7.429688 dB";

    #[test]
    fn parse_text() {
        make_frame!(TextFrame, TIT2_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Utf16);
        assert_eq!(frame.text[0], "I Swallowed Hard, Like I Understood");

        make_frame!(TextFrame, TCON_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Latin1);

        assert_eq!(frame.text[0], "Post-Rock");
        assert_eq!(frame.text[1], "Electronica");
    }

    #[test]
    fn parse_credits() {
        make_frame!(CreditsFrame, TMCL_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Latin1);
        assert_eq!(frame.people["Bassist"], "John Smith");
        assert_eq!(frame.people["Violinist"], "Vanessa Evans");
    }

    #[test]
    fn parse_txxx() {
        make_frame!(UserTextFrame, TXXX_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Latin1);
        assert_eq!(frame.desc, "replaygain_track_gain");
        assert_eq!(frame.text[0], "-7.429688 dB");
    }
}
