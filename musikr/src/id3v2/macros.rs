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

#[macro_export]
macro_rules! tipl_frame {
    ($($role:expr => $people:expr),+ $(,)?) => {
        tipl_frame!(crate::string::Encoding::default(), $($role, $people)*)
    };
    ($enc:expr, $($role:expr => $people:expr),+ $(,)?) => {
        {
            let mut frame = crate::id3v2::frames::CreditsFrame::new_tipl();
            $(frame.people.insert(String::from($role), String::from($people));)*
            frame
        }
    }
}

#[macro_export]
macro_rules! tmcl_frame {
    ($($role:expr => $people:expr),+ $(,)?) => {
        tmcl_frame!(crate::string::Encoding::default(), $($role => $people)*)
    };
    ($enc:expr, $($role:expr => $people:expr),+ $(,)?) => {
        {
            let mut frame = crate::id3v2::frames::CreditsFrame::new_tmcl();
            frame.encoding = $enc;
            $(frame.people.insert(String::from($role), String::from($people));)*
            frame
        }
    }
}

#[macro_export]
macro_rules! url_frame {
    ($id:expr, $url:expr) => {{
        let mut frame = crate::id3v2::frames::UrlFrame::new(FrameId::new($id));
        frame.url = String::from($url);
        frame
    }};
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
