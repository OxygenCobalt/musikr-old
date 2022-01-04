/// Generates an ID3v2 [`TextFrame`](crate::id3v2::frames::TextFrame) from the given elements.
///
/// This macro allows an ID3v2 text frame to be created more ergonomically. All rules from
/// [`TextFrame::new`](crate::id3v2::frames::TextFrame::new) apply to this macro.
///
/// # Examples
/// Create a frame with an ID and a list of text strings:
///
/// ```
/// use musikr::{text_frame, id3v2::frames::Frame};
///
/// let frame = text_frame! {
///     b"TIT2", ["Song Title"]
/// };
///
/// assert_eq!(frame.id(), b"TIT2");
/// assert_eq!(frame.text[0], "Song Title");
/// ```
///
/// Create a frame with an ID, an [`Encoding`](crate::core::Encoding), and a list of text strings:
///
/// ```
/// use musikr::{text_frame, id3v2::frames::Frame, string::Encoding};
///
/// let frame = text_frame! {
///     b"TLAN",
///     Encoding::Utf16,
///     ["eng", "deu"]
/// };
///
/// assert_eq!(frame.id(), b"TLAN");
/// assert_eq!(frame.encoding, Encoding::Utf16);
/// assert_eq!(frame.text[0], "eng");
/// assert_eq!(frame.text[1], "deu");
/// ```
#[macro_export]
macro_rules! text_frame {
    ($id:expr) => {
        {
            $crate::id3v2::frames::TextFrame::new($crate::id3v2::frames::FrameId::new($id))
        }
    };
    ($id:expr, [$($text:expr),+ $(,)?]) => {
        {
            let mut frame = $crate::id3v2::frames::TextFrame::new($crate::id3v2::frames::FrameId::new($id));
            frame.text = vec![$(String::from($text),)*];
            frame
        }

    };
    ($id:expr, $enc:expr, [$($text:expr),+ $(,)?]) => {
        {
            let mut frame = $crate::id3v2::frames::TextFrame::new($crate::id3v2::frames::FrameId::new($id));
            frame.encoding = $enc;
            frame.text = vec![$(String::from($text),)*];
            frame
        }
    };
}

/// Generates an ID3v2 [`CreditsFrame`](crate::id3v2::frames::CreditsFrame) from the given elements.
///
/// This macro allows an ID3v2 credits frame to be created more ergonomically. All rules from
/// [`CreditsFrame::new`](crate::id3v2::frames::CreditsFrame::new) apply to this macro.
///
/// # Examples
/// Create a frame with an ID and a list of text strings:
///
/// ```
/// use musikr::{credits_frame, id3v2::frames::Frame};
///
/// let frame = credits_frame! {
///     b"TMCL",
///     "Bassist" => "Person 1",
///     "Violinist" => "Person 2"
/// };
///
/// assert_eq!(frame.id(), b"TMCL");
/// assert_eq!(frame.people["Bassist"], "Person 1");
/// assert_eq!(frame.people["Violinist"], "Person 2");
/// ```
///
/// Create a frame with an ID, an [`Encoding`](crate::core::Encoding), and a list of text strings:
///
/// ```
/// use musikr::{credits_frame, id3v2::frames::Frame, string::Encoding};
///
/// let frame = credits_frame! {
///     b"TMCL",
///     Encoding::Utf16,
///     "Bassist" => "Person 1",
///     "Violinist" => "Person 2"
/// };
///
/// assert_eq!(frame.id(), b"TMCL");
/// assert_eq!(frame.encoding, Encoding::Utf16);
/// assert_eq!(frame.people["Bassist"], "Person 1");
/// assert_eq!(frame.people["Violinist"], "Person 2");
/// ```
#[macro_export]
macro_rules! credits_frame {
    ($id:expr, $($role:expr => $people:expr),+ $(,)?) => {
        {
            let mut frame = $crate::id3v2::frames::CreditsFrame::new($crate::id3v2::frames::FrameId::new($id));
            $(frame.people.insert(String::from($role), String::from($people));)*
            frame
        }
    };
    ($id:expr, $enc:expr, $($role:expr => $people:expr),+ $(,)?) => {
        {
            let mut frame = $crate::id3v2::frames::CreditsFrame::new($crate::id3v2::frames::FrameId::new($id));
            frame.encoding = $enc;
            $(frame.people.insert(String::from($role), String::from($people));)*
            frame
        }
    }
}

/// Generates an ID3v2 [`UrlFrame`](crate::id3v2::frames::UrlFrame) from the given elements.
///
/// This macro allows an ID3v2 url frame to be created more ergonomically. All rules from
/// [`UrlFrame::new`](crate::id3v2::frames::UrlFrame::new) apply to this macro.
///
/// # Examples
/// ```
/// use musikr::{url_frame, id3v2::frames::Frame};
///
/// let frame = url_frame! {
///     b"WOAR",
///     "https://test.com"
/// };
///
/// assert_eq!(frame.id(), b"WOAR");
/// assert_eq!(frame.url, "https://test.com");
/// ```
#[macro_export]
macro_rules! url_frame {
    ($id:expr, $url:expr) => {{
        let mut frame =
            $crate::id3v2::frames::UrlFrame::new($crate::id3v2::frames::FrameId::new($id));
        frame.url = String::from($url);
        frame
    }};
}

// --- Internal macros ---

macro_rules! is_id {
    ($id:expr, $($ids:expr),+ $(,)?) => {
        {
            if let $(| $ids)* = $id.as_ref() {
                true
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
macro_rules! make_frame {
    ($dty:ty, $data:expr, $dest:ident) => {
        make_frame!($dty, $data, crate::id3v2::tag::Version::V24, $dest)
    };

    ($dty:ty, $data:expr, $ver:expr, $dest:ident) => {
        let parsed = crate::id3v2::frames::parse(
            &crate::id3v2::tag::TagHeader::with_version($ver),
            &mut crate::core::io::BufStream::new($data),
            &crate::id3v2::frames::DefaultFrameParser { strict: true },
        )
        .unwrap();

        let frame = if let crate::id3v2::frames::ParsedFrame::Frame(frame) = parsed {
            frame
        } else {
            panic!("cannot parse frame: {:?}", parsed)
        };

        let $dest = frame.downcast::<$dty>().unwrap();
    };
}

#[cfg(test)]
macro_rules! assert_render {
    ($frame:expr, $data:expr) => {
        assert!(!$frame.is_empty());
        assert_eq!(
            crate::id3v2::frames::render(
                &crate::id3v2::tag::TagHeader::with_version(crate::id3v2::tag::Version::V24),
                &$frame
            )
            .unwrap(),
            $data
        )
    };
}
