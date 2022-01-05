//! Text information frames.
//!
//! Text frames store specific text information, such as a song name. ID3v2 handles text information differs from
//! other metadata formats, and so the implementation is split across multiple distinct datatypes. However, a
//! couple of details are common across all implementations:
//!
//! - Text frames expose an encoding that will be used when the frame is written. More information can be found
//! in [`Encoding`](crate::core::Encoding).
//! - A text frame implementation can correspond to multiple Frame IDs.
//! - A text frame can contain more than one string.
//!
//! # Quirks
//!
//! - Certain text frames may be an iTunes extension or only exist in a specific ID3v2 version. If this is the case,
//! then it will be marked accordingly.
//! - According to the standard, ID3v2.3 text frames cannot have multiple fields delimited by a null terminator.
//! While musikr does not enforce this restriction, some taggers might.
//! - `UserTextFrame` is not meant to have multiple fields, however the other major tagging libraries all seem to
//! enable this, so musikr implements it regardless.

use crate::core::io::BufStream;
use crate::core::string::{self, Encoding};
use crate::id3v2::frames::{encoding, Frame, FrameId, Language};
use crate::id3v2::{ParseResult, TagHeader};
use log::{info, warn};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

/// Specific text-based metadata.
///
/// This frame actually corresponds to many different frames, many with their own invariants that musikr
/// does not enforce for simplicity. **Failure to follow these invariants may result in unexpected behavior
/// in other programs or in musikr itself.**
///
/// #### Word Frames
///
/// Frames that encode character-based text. Some frames might require a specific format, but musikr does not
/// enforce those invariants.
///
/// ```text
/// TALB Album/movie/show title
/// TCOM Composer
/// TCON Content type, better known as a genre. Its recommended to write full strings to this frame instead of ID3v1 genres.
/// TCOP Copyright message or notice for this audio. Must be prefixed with a year and a space.
/// TENC Person/organization that encoded this audio file
/// TEXT Lyricist(s)/Writer(s) for this audio
/// TFLT Audio type/encoding. [More Info](https://id3.org/id3v2.3.0#TFLT)
/// TIT1 Category of audio [ex. "Classical Music" or "Weather"]
/// TIT2 Title/Song Name/Content Description [ex. "Unspoken", "Hurricane Elsa"]
/// TIT3 Refinement/description [ex. "Performed at X"]
/// TKEY Initial key of this song [e.x A#]
/// TLAN The ISO-639-2 Language(s) spoken in the audio. Using [`Language`](crate::id3v2::frames::Language) here is recommended to ensure valid values.
/// TMED Type of media the audio was derived from. [More Info](https://id3.org/id3v2.3.0#TMED)
/// TOAL Original album title for this audio [e.g for song covers or remixes]
/// TOFN Original filename for this audio [see TOAL]
/// TOLY Original lyricist(s)/text writer(s) [see TOAL]
/// TOPE Original artist(s)/performer(s) [see TOAL]
/// TOWN Owner/licensee of this audio
/// TPE1 Main artist/performer/group [e.x Radiohead]
/// TPE2 Additional information about the performers, such as an album artist
/// TPE3 Conductor
/// TPE4 Remixer(s)/interpreter(s)
/// TPUB Publisher of the audio
/// TRSN The internet radio station the audio is streamed from
/// TRSO The owner of the internet radio station [See TRSN]
/// TSRC ISRC (international standard recording code)
/// TSSE Software/Hardware/Settings used for encoding
///
/// TRDA [ID3v2.3] A list of recording dates [e.x "June 16th"]
/// TMOO [ID3v2.4] Mood [e.x "Sad", "Atmospheric"]
/// TPRO [ID3v2.4] Production/copyright holder of this audio. Must begin with a year and a space.
/// TSOA [ID3v2.4] Album title that should be used for sorting [e.g TALB "The Eraser" -> TSOA "Eraser"]
/// TSOP [ID3v2.4] Artist name that should be used for sorting [See TPE1 "The Beatles" -> TSOP "Beatles"]
/// TSOT [ID3v2.4] Title that should be used for sorting [e.x TIT2 "The Axe" -> TSOT "Axe"]
/// TSST [ID3v2.4] Subtitle that this track belongs to
/// TSO2 [iTunes]  Album artist that should be used for sorting [See TSOP]
/// TSOC [iTunes]  Composer that should be used for sorting [See TSOP]
/// TCAT [iTunes]  Podcast Category
/// TDES [iTunes]  Podcast description
/// TGID [iTunes]  Podcast Identifier
/// TKWD [iTunes]  Podcast Keyword
/// WFED [iTunes]  Podcast Feed URL [Actually a text frame]
/// MVNM [iTunes]  Movement name
/// GRP1 [iTunes]  Grouping
/// ```
///
/// #### Numeric Frames
/// These frames contain numeric strings, or strings that should only contain the letters 0-9. Musikr does
/// not enforce this restriction however, since other taggers will not follow this rule and put other information
/// in anyway. It's recommended not to assume that all frames will be numeric when parsing, but to enforce the
/// invariant when writing new frames.
///
/// **Note:** When upgrading, musikr will only extract numeric information from these frames when upgrading.
/// Malformed frames may result in lost information.
///
/// ```text
/// TBPM The BPM [Beats per minute] of the audio
/// TDLY The delay between the end of this song and the next song in a playlist, in millis
/// TLEN The length of this audio, in millis
/// TYER [ID3v2.3] The year(s) this audio was recorded, formatted as YYYY. Must be at least 4 characters.
/// TDAT [ID3v2.3] The date(s) this audio was recorded, formatted as MMDD. Must be 4 characters.
/// TIME [ID3v2.3] The time(s) this audio was recorded, formatted as HHMM. Must be 4 characters.
/// TORY [ID3v2.3] The year this audio was released, formatted as YYYY. Must be at least 4 characters.
/// TSIZ [ID3v2.3] The size of the audio, in bytes.
/// TCMP [iTunes]  Marks if this file is part of a compilation. 1 if yes, 0 if no.
/// ```
///
/// #### Numeric Part Frames
/// These are subset of numeric frames that are numeric strings `NN` that can be optionally
/// extended with a "total" value, forming `NN/TT`. Like numeric frames, musikr does not enforce these
/// invariants.
///
/// ```text
/// TPOS          The part of an set this track comes from, such as a collection of albums
/// TRCK          The track number of this audio
/// MVIN [iTunes] The "Movement Number" of this audio
/// ```
///
/// Timestamp Frames:
///
/// These are frames that represent a timestamp, formatted as `YYYY-MM-DDTHH:MM:SS`. Precision can be
/// tuned, meaning that `YYYY`, `YYYY-MM`, `YYYY-MM-DD`, `YYYY-MM-DDTHH`, `YYYY-MM-DDTHH:MM`,
/// `YYYY-MM-DD-THH:MM:SS` are all valid timestamps.
///
/// Its recommended to use these frames instead of the legacy TYER, TDAT, TIME, and TORY frames,
/// as they will be automatically turned into those counterparts when saved.
///
/// **Note:** When upgrading, musikr will assume that thesse frames will only contain a valid timestamp
/// when trying to upgrade frames. Malformed frames may result in lost information.
///
/// ```text
/// TDRC [ID3v2.4] Time this file was recorded
/// TDEN [ID3v2.4] Time this file was encoded
/// TDRL [ID3v2.4] Time this file was released
/// TDOR [ID3v2.4] Time this file was originally released
/// TDTG [ID3v2.4] Time this file was tagged
/// ```
#[derive(Debug, Clone)]
pub struct TextFrame {
    frame_id: FrameId,
    /// The encoding that the frame will use to write `text`.
    pub encoding: Encoding,
    /// The text content of this frame. If the field is empty or if all strings
    /// in the field are empty, then the frame will not be written.
    pub text: Vec<String>,
}

impl TextFrame {
    /// Creates a new instance of this frame from `frame_id`.
    ///
    /// For a more ergonomic instantiation of this frame, try the
    /// [`text_frame!`](crate::text_frame) macro.
    ///
    /// # Panics
    ///
    /// This function will panic if the Frame ID is not a valid `TextFrame` ID.
    pub fn new(frame_id: FrameId) -> Self {
        // Ensure the ID is valid for this frame.
        if !Self::is_id(frame_id) {
            panic!("expected a valid text frame id, found {}", frame_id);
        }

        Self {
            frame_id,
            encoding: Encoding::default(),
            text: Vec::new(),
        }
    }

    /// Returns if `frame_id` will is valid for this frame.
    ///
    /// See the the overall frame documentation for a list of valid `TextFrame` ID.
    #[rustfmt::skip]
    pub fn is_id(frame_id: FrameId) -> bool {
        is_id!(
            // Text
            frame_id, b"TALB", b"TCOM", b"TCON", b"TCOP", b"TENC", b"TEXT", b"TFLT", b"TIT1",
            b"TIT2", b"TIT3", b"TKEY", b"TLAN", b"TMED", b"TOAL", b"TOFN", b"TOLY", b"TOPE",
            b"TOWN", b"TPE1", b"TPE2", b"TPE3", b"TPE4", b"TPUB", b"TRSN", b"TRSO", b"TSRC",
            b"TSSE", b"TRDA", b"TMOO", b"TPRO", b"TSOA", b"TSOP", b"TSOT", b"TSST", b"TSO2",
            b"TSOC", b"TCAT", b"TDES", b"TGID", b"TKWD", 
            // Numeric
            b"TLEN", b"TYER", b"TDAT", b"TIME", b"TORY", b"TSIZ", b"TCMP", b"TDLY", b"TBPM",
            // Numeric part
            b"TPOS", b"TRCK",
            // Timestamps
            b"TDEN", b"TDOR", b"TDRC", b"TDRL", b"TDTG",
            // iTunes WFED [Podcast URL], MVNM [Movement Name], MVIN [Movement Number],
            // and GRP1 [Grouping] are all actually text frames
            b"WFED", b"MVNM", b"MVIN", b"GRP1"
        )
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
        fmt_text(f, &self.text)
    }
}

/// Text information not represented by other frames.
///
/// This frame can be used to add program-defined tags without having to create a new frame
/// implementation. The only ID for this frame is `TXXX`. Identifying information should be
/// put into the [`desc`](UserTextFrame.desc) field.
///
/// Notable examples of these frames include:
/// - ReplayGain tags (ex. `replaygain_track_gain`)
/// - MusicBrainz tags
#[derive(Default, Debug, Clone)]
pub struct UserTextFrame {
    /// The encoding that the frame will use to write `desc` and `text`.
    pub encoding: Encoding,
    /// A description of the contents in this frame. This should be unique
    /// and non-empty.
    pub desc: String,
    /// The text content of this frame. If the field is empty or if all strings
    /// in the field are empty, then the frame will not be written.
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
        self.desc.is_empty() && self.text.len() == 0
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
        fmt_text(f, &self.text)
    }
}

/// A mapping between involved people and their roles.
///
/// All entries in this frame are kept in alphabetical order, by role. A role cannot
/// be mapped to multiple strings. It's recommended that `TIPL` and `TMCL` are used
/// over `IPLS`, as those frames will automatically be downgraded to `IPLS` if the
/// tag is saved with ID3v2.3.
///
/// ```text
/// IPLS [ID3v2.3] Maps between a role and a list of people for that role
/// TIPL [ID3v2.4] Maps between a role and a list of people for that role
/// TMCL [ID3v2.4] Maps between an instrument and the people who played that instrument
/// ```
///
/// **Note:** Regardless of the version, `TIPL` and `TMCL` will always be used to represent the
/// frame in a [`FrameMap`](crate::id3v2::collections::FrameMap).
#[derive(Debug, Clone)]
pub struct CreditsFrame {
    frame_id: FrameId,
    /// The encoding that the frame will use to write `people`.
    pub encoding: Encoding,
    /// A mapping between roles and people. Multiple people can be delimited with
    /// a comma or similar separator.
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
        match self.frame_id.as_ref() {
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
    /// Creates a new instance of this frame from `frame_id`.
    ///
    /// For a more ergonomic instantiation of this frame, try the
    /// [`credits_frame!`](crate::credits_frame) macro.
    ///
    /// # Panics
    ///
    /// This function will panic if the Frame ID is not a valid `CreditsFrame` ID.
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

    /// Returns if `frame_id` will is valid for this frame.
    ///
    /// See the the overall frame documentation for a list of valid Frame IDs.
    pub fn is_id(frame_id: FrameId) -> bool {
        is_id!(frame_id, b"IPLS", b"TIPL", b"TMCL")
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

/// A frame that contains a comment.
///
/// This frame differs from [`UserTextFrame`](UserTextFrame) in that instead of containing
/// program-defined text information, the frame instead contains user-defined text information
/// without any specific format.
///
/// Despite of this, this frame is still used interchangeably with [`UserTextFrame`](UserTextFrame),
/// such as with `iTunNORM` comments. One should be prepared to parse custom information from either
/// of these frames if the situation arises.
#[derive(Default, Debug, Clone)]
pub struct CommentsFrame {
    /// The encoding that the frame will use to write `desc` and `text`.
    pub encoding: Encoding,
    /// The language that `desc` and `text` is written in.
    pub lang: Language,
    /// The description of the text, usually written by a user. Can be empty.
    pub desc: String,
    /// The text contents of this frame.
    pub text: String,
}

impl CommentsFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let lang = Language::try_new(&stream.read_array()?).unwrap_or_default();
        let desc = string::read_terminated(encoding, stream);
        let text = string::read(encoding, stream);

        Ok(Self {
            encoding,
            lang,
            desc,
            text,
        })
    }
}

impl Frame for CommentsFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"COMM")
    }

    fn key(&self) -> String {
        format!["COMM:{}:{}", self.desc, self.lang]
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.version());
        result.push(encoding::render(encoding));
        result.extend(&self.lang);

        result.extend(string::render_terminated(encoding, &self.desc));
        result.extend(string::render(encoding, &self.text));

        result
    }
}

impl Display for CommentsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.text]
    }
}

fn fmt_text<D: Display>(f: &mut Formatter, text: &[D]) -> fmt::Result {
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

    const COMM_DATA: &[u8] = b"COMM\x00\x00\x00\x14\x00\x00\
                                \x03\
                                eng\
                                Description\x00\
                                Text";
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

    #[test]
    fn parse_comm() {
        make_frame!(CommentsFrame, COMM_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Utf8);
        assert_eq!(frame.lang, b"eng");
        assert_eq!(frame.desc, "Description");
        assert_eq!(frame.text, "Text");
    }

    #[test]
    fn render_comm() {
        let frame = CommentsFrame {
            encoding: Encoding::Utf8,
            lang: Language::new(b"eng"),
            desc: String::from("Description"),
            text: String::from("Text"),
        };

        assert_render!(frame, COMM_DATA);
    }
}
