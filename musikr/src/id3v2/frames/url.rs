//! URL information frames.
//!
//! URL frames contain a link to a webpage. The structure of these frames are similar to 
//! [text frames](crate::id3v2::frames::text), however with some key differences:
//!
//! - The encoding of a frame is always [Encoding::Latin1](crate::string::Encoding::Latin1).
//! - There cannot be multiple URLs in a frame.
//!
//! Musikr does not ensure the validity of the URLs in a frame. It is up to the user to determine if URL
//! validation is necessary for their use case.

use crate::core::io::BufStream;
use crate::id3v2::frames::{encoding, Frame, FrameId};
use crate::id3v2::{ParseResult, TagHeader};
use crate::core::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

/// Specific URL metadata.
///
/// All valid URL frames are listed in the following:
///
/// ```text
/// WCOM A page where the album or song can be bought.
/// WCOP A page containing a full copyright notice. 
/// WOAF The official webpage for the media file.
/// WOAR The official artist/performer webpage.
/// WOAS The official webpage for the source of the media.
/// WORS The webpage for the radio station.
/// WPAY A page that handles payment for the file.
/// WPUB The official page for the publisher.
/// ```
///
/// **Note:** Do not try to use `WFED` with this frame. iTunes actually treats the frame like
/// a [TextFrame](crate::id3v2::frames::text::TextFrame).
#[derive(Debug, Clone)]
pub struct UrlFrame {
    frame_id: FrameId,
    /// The URL of this string.
    pub url: String,
}

impl UrlFrame {
    /// Creates a new instance of this frame from `frame_id`.
    ///
    /// For a more ergonomic instantiation of this frame, try the 
    /// [`url_frame!`](crate::url_frame) macro.
    ///
    /// # Panics
    ///
    /// This function will panic if the Frame ID is not a valid `UrlFrame` ID.
    pub fn new(frame_id: FrameId) -> Self {
        if !Self::is_id(frame_id) {
            panic!("expected a valid url frame id, found {}", frame_id)
        }

        Self {
            frame_id,
            url: String::new(),
        }
    }

    pub(crate) fn parse(frame_id: FrameId, stream: &mut BufStream) -> ParseResult<Self> {
        let url = string::read(Encoding::Utf8, stream);

        Ok(Self { frame_id, url })
    }

    pub fn is_id(frame_id: FrameId) -> bool {
        is_id!(frame_id, b"WCOM", b"WCOP", b"WOAF", b"WOAR", b"WOAS", b"WORS", b"WPAY", b"WPUB")
    }
}

impl Frame for UrlFrame {
    fn id(&self) -> FrameId {
        self.frame_id
    }

    fn key(&self) -> String {
        self.id().to_string()
    }

    fn is_empty(&self) -> bool {
        self.url.is_empty()
    }

    fn render(&self, _: &TagHeader) -> Vec<u8> {
        string::render(Encoding::Latin1, &self.url)
    }
}

impl Display for UrlFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.url]
    }
}

/// URL information not represented by other frames.
///
/// This frame can be used to add program-specific tags without having to create a new frame
/// implementation. The only ID for this frame is `WXXX`. Identifying information should be
/// put into the [`desc`](UserUrlFrame.desc) field.
#[derive(Default, Debug, Clone)]
pub struct UserUrlFrame {
    pub encoding: Encoding,
    pub desc: String,
    pub url: String,
}

impl UserUrlFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let desc = string::read_terminated(encoding, stream);
        let url = string::read(Encoding::Latin1, stream);

        Ok(Self {
            encoding,
            desc,
            url,
        })
    }
}

impl Frame for UserUrlFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"WXXX")
    }

    fn key(&self) -> String {
        format!["WXXX:{}", self.desc]
    }

    fn is_empty(&self) -> bool {
        self.url.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.version());
        result.push(encoding::render(self.encoding));

        result.extend(string::render_terminated(encoding, &self.desc));
        result.extend(string::render(Encoding::Latin1, &self.url));

        result
    }
}

impl Display for UserUrlFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.url]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WOAR_DATA: &[u8] = b"WOAR\x00\x00\x00\x13\x00\x00\
                              https://fourtet.net";

    const WXXX_DATA: &[u8] = b"WXXX\x00\x00\x00\x24\x00\x00\
                               \x03\
                               ID3v2.3.0\0\
                               https://id3.org/id3v2.3.0";

    #[test]
    fn parse_url() {
        make_frame!(UrlFrame, WOAR_DATA, frame);

        assert_eq!(frame.url, "https://fourtet.net");
    }

    #[test]
    fn parse_wxxx() {
        make_frame!(UserUrlFrame, WXXX_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Utf8);
        assert_eq!(frame.desc, "ID3v2.3.0");
        assert_eq!(frame.url, "https://id3.org/id3v2.3.0");
    }

    #[test]
    fn render_url() {
        let frame = crate::url_frame! {
            b"WOAR"; "https://fourtet.net"
        };

        assert_render!(frame, WOAR_DATA);
    }

    #[test]
    fn render_wxxx() {
        let frame = UserUrlFrame {
            encoding: Encoding::Utf8,
            desc: String::from("ID3v2.3.0"),
            url: String::from("https://id3.org/id3v2.3.0"),
        };

        assert_render!(frame, WXXX_DATA);
    }
}
