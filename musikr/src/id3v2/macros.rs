/// Generates a [`TextFrame`](crate::id3v2::frames::TextFrame) from the given elements.
///
/// `text_frame!` allows an ID3v2 text frame to be created similarly to a struct definition, like other frame types.
/// There are two forms of this macro:
///
/// - Create a [`TextFrame`](crate::id3v2::frames::TextFrame) with an ID and a list of text strings
///
/// ```
/// use musikr::{text_frame, id3v2::frames::Frame};
///
/// let frame = text_frame! {
///     b"TIT2"; "Song Title"
/// };
///
/// assert_eq!(frame.id(), b"TIT2");
/// assert_eq!(frame.text[0], "Song Title");
/// ```
///
/// - Create a [`TextFrame`](crate::id3v2::frames::TextFrame) with an ID, an [`Encoding`](crate::string::Encoding),
/// and a list of text strings
///
/// ```
/// use musikr::{text_frame, id3v2::frames::Frame, string::Encoding};
///
/// let frame = text_frame! {
///     b"TLAN",
///     Encoding::Utf16,
///     "eng", "deu"
/// };
///
/// assert_eq!(frame.id(), b"TLAN");
/// assert_eq!(frame.encoding, Encoding::Utf16);
/// assert_eq!(frame.text[0], "eng");
/// assert_eq!(frame.text[1], "deu");
/// ```
///
/// All rules from [`TextFrame::new`](crate::id3v2::frames::TextFrame::new) apply to this macro.
#[macro_export]
macro_rules! text_frame {
    ($id:expr) => {
        {
            $crate::id3v2::frames::TextFrame::new($crate::id3v2::frames::FrameId::new($id))
        }
    };
    ($id:expr; $($text:expr),+ $(,)?) => {
        {
            let mut frame = $crate::id3v2::frames::TextFrame::new($crate::id3v2::frames::FrameId::new($id));
            frame.text = vec![$(String::from($text),)*];
            frame
        }

    };
    ($id:expr, $enc:expr, $($text:expr),+ $(,)?) => {
        {
            let mut frame = $crate::id3v2::frames::TextFrame::new($crate::id3v2::frames::FrameId::new($id));
            frame.encoding = $enc;
            frame.text = vec![$(String::from($text),)*];
            frame
        }
    };
}

#[macro_export]
macro_rules! credits_frame {
    ($id:expr, $($role:expr => $people:expr),+ $(,)?) => {
        {
            let mut frame = $crate::id3v2::frames::CreditsFrame::new(crate::id3v2::frames::FrameId::new($id));
            $(frame.people.insert(String::from($role), String::from($people));)*
            frame
        }
    };
    ($id:expr, $enc:expr, $($role:expr => $people:expr),+ $(,)?) => {
        {
            let mut frame = $crate::id3v2::frames::CreditsFrame::new(crate::id3v2::frames::FrameId::new($id));
            frame.encoding = $enc;
            $(frame.people.insert(String::from($role), String::from($people));)*
            frame
        }
    }
}

/// Generates a new [`UrlFrame`](crate::id3v2::frames::UrlFrame) from the given elements.
///
/// `url_frame!` allows an ID3v2 url frame to be created similarly to a struct definition, like other
/// frame types.
///
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
///
/// All rules from [`UrlFrame::new`](crate::id3v2::frames::UrlFrame::new) apply to this macro.
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
            if let $(| $ids)* = $id.inner() {
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
        )
        .unwrap();

        let frame = if let crate::id3v2::frames::FrameResult::Frame(frame) = parsed {
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
