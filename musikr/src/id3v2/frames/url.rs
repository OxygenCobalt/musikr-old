use crate::core::io::BufStream;
use crate::id3v2::frames::{encoding, Frame, FrameId};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct UrlFrame {
    frame_id: FrameId,
    pub url: String,
}

impl UrlFrame {
    /// Creates a new instance of this frame from `frame_id`.
    ///
    /// ```
    /// use musikr::id3v2::frames::{Frame, FrameId, UrlFrame};
    ///
    /// let frame = UrlFrame::new(FrameId::new(b"WOAR"));
    /// assert_eq!(frame.id(), b"WOAR");
    /// ```
    ///
    /// # Panics
    ///
    /// This function will panic if the Frame ID is not a valid URL frame ID. Valid frame IDs are:
    /// WCOM`, `WCOP`, `WOAF`, `WOAR`, `WOAS`, `WORS`, `WPAY`, and `WPUB`
    ///
    /// For a more struct-like instantiation of this frame, try the [`url_frame!`](crate::url_frame)
    /// macro.
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

#[derive(Debug, Clone)]
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

impl Default for UserUrlFrame {
    fn default() -> Self {
        Self {
            encoding: Encoding::default(),
            desc: String::new(),
            url: String::new(),
        }
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
            b"WOAR",
            "https://fourtet.net"
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
