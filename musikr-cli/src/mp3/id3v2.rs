use crate::show::{DisplayName, DisplayTag, TagFilter};
use musikr::id3v2::{
    Tag, frames::{CommentsFrame, Frame, FrameId, UserTextFrame, UserUrlFrame}
};

pub fn show(tag: Tag, filter: TagFilter) -> Vec<DisplayTag> {
    let mut tags = Vec::new();
    let (filter_names, filter_ids) = process_filter(filter);

    for frame in tag.frames.values() {
        let display_tag = transform_frame(frame);

        if !filter_ids.is_empty() || !filter_names.is_empty() {
            if filter_ids.contains(&frame.id()) {
                // Filter case 1: A manual !XXXX id was specified.
                tags.push(display_tag)
            } else {
                // Filter case 2: A readable name was specified.
                // This could be in the form of a simple tag name like "title",
                // the name of a custom tag like "replaygain_track_gain", or the
                // name of a specific tag variation, like "comment (xyz)".
                let name_matches = match display_tag.name {
                    DisplayName::Name(ref name) => filter_names.contains(name),
                    DisplayName::Custom(ref name, ref custom) => {
                        filter_names.contains(name) || filter_names.contains(&custom.as_str())
                    }
                    DisplayName::Unknown(_) => false,
                };

                if name_matches {
                    tags.push(display_tag)
                }
            }
        } else {
            tags.push(display_tag)
        }
    }

    tags.sort();

    tags
}

fn process_filter(filter: TagFilter) -> (Vec<&str>, Vec<FrameId>) {
    let mut filter_names = Vec::new();
    let mut filter_ids = Vec::new();

    if let Some(tags) = filter {
        for tag in tags {
            if let Some(Ok(id)) = tag.strip_prefix('^').map(|id| id.parse::<FrameId>()) {
                // User inputted a raw frame ID
                filter_ids.push(id);
            } else {
                filter_names.push(tag);
            }
        }
    }

    (filter_names, filter_ids)
}

/// --- FRAME TRANSFORMATION ---

fn transform_frame(frame: &dyn Frame) -> DisplayTag {
    for analogue in SHOW_ANALOGUES {
        for id in analogue.ids {
            if frame.id() == *id {
                return (analogue.transform)(analogue.name, frame);
            }
        }
    }

    DisplayTag {
        name: DisplayName::Unknown(frame.id().to_string()),
        value: frame.to_string(),
    }
}

// --- TRANSFORMATION ---

struct Analogue<F: Fn(&'static str, &dyn Frame) -> DisplayTag> {
    ids: &'static [&'static [u8; 4]],
    name: &'static str,
    transform: F,
}

type Transform = fn(&'static str, &dyn Frame) -> DisplayTag;

// All ID3v2 tags that musikr knows a name for.
// This list is in-progress, more will be added as time progresses.
#[rustfmt::skip]
static SHOW_ANALOGUES: &[Analogue<Transform>] = &[
    Analogue { ids: &[b"TALB"],name: "album", transform: plain_transform },
    Analogue { ids: &[b"TCOM"], name: "composer", transform: plain_transform },
    Analogue { ids: &[b"TCON"], name: "genre", transform: plain_transform },
    Analogue { ids: &[b"TCOP"], name: "copyright", transform: plain_transform },
    Analogue { ids: &[b"TENC"], name: "encoded_by", transform: plain_transform },
    Analogue { ids: &[b"TEXT"], name: "writer", transform: plain_transform },
    Analogue { ids: &[b"TFLT"], name: "file_type", transform: plain_transform },
    Analogue { ids: &[b"TIT1"], name: "category", transform: plain_transform },
    Analogue { ids: &[b"TIT2"], name: "title", transform: plain_transform },
    Analogue { ids: &[b"TIT3"], name: "subtitle", transform: plain_transform },
    Analogue { ids: &[b"TKEY"], name: "initial_key", transform: plain_transform },
    Analogue { ids: &[b"TLAN"], name: "language", transform: plain_transform },
    Analogue { ids: &[b"TMED"], name: "media_type", transform: plain_transform },
    Analogue { ids: &[b"TOAL"], name: "original_album", transform: plain_transform },
    Analogue { ids: &[b"TOFN"], name: "original_filename", transform: plain_transform },
    Analogue { ids: &[b"TOLY"], name: "original_writer", transform: plain_transform },
    Analogue { ids: &[b"TOPE"], name: "original_artist", transform: plain_transform },
    Analogue { ids: &[b"TOWN"], name: "owner", transform: plain_transform },
    Analogue { ids: &[b"TPE1"], name: "artist", transform: plain_transform },
    Analogue { ids: &[b"TPE2"], name: "album_artist", transform: plain_transform },
    Analogue { ids: &[b"TPE3"], name: "conductor", transform: plain_transform },
    Analogue { ids: &[b"TPE4"], name: "remixer", transform: plain_transform },
    Analogue { ids: &[b"TPUB"], name: "publisher", transform: plain_transform },
    Analogue { ids: &[b"TRSN"], name: "station", transform: plain_transform },
    Analogue { ids: &[b"TRSO"], name: "station_owner", transform: plain_transform },
    Analogue { ids: &[b"TSRC"], name: "isrc", transform: plain_transform },
    Analogue { ids: &[b"TSSE"], name: "encoding", transform: plain_transform },
    Analogue { ids: &[b"TRDA"], name: "recording_dates", transform: plain_transform }, //[ID3v2.3]
    Analogue { ids: &[b"TMOO"], name: "mood", transform: plain_transform }, // [ID3v2.4]
    Analogue { ids: &[b"TPRO"], name: "copyright_notice", transform: plain_transform }, // [ID3v2.4]
    Analogue { ids: &[b"TSOA"], name: "sort_album", transform: plain_transform }, // [ID3v2.4]
    Analogue { ids: &[b"TSOP"], name: "sort_artist", transform: plain_transform }, // [ID3v2.4]
    Analogue { ids: &[b"TSOT"], name: "sort_title", transform: plain_transform }, // [ID3v2.4]
    Analogue { ids: &[b"TSST"], name: "sort_subtitle", transform: plain_transform }, //
    Analogue { ids: &[b"TSO2"], name: "sort_album_artist", transform: plain_transform }, // [iTunes]
    Analogue { ids: &[b"TSOC"], name: "sort_composer", transform: plain_transform }, // [iTunes]
    Analogue { ids: &[b"TCAT"], name: "podcast_category", transform: plain_transform }, // [iTunes]
    Analogue { ids: &[b"TDES"], name: "podcast_desc", transform: plain_transform }, // [iTunes]
    Analogue { ids: &[b"TGID"], name: "podcast_id", transform: plain_transform }, // [iTunes]
    Analogue { ids: &[b"TKWD"], name: "podcast_keyword", transform: plain_transform }, // [iTunes]
    Analogue { ids: &[b"WFED"], name: "podcast_url", transform: plain_transform }, // [iTunes]
    Analogue { ids: &[b"MVNM"], name: "movement_name", transform: plain_transform }, // [iTunes]
    Analogue { ids: &[b"GRP1"], name: "grouping", transform: plain_transform }, // [iTunes]
    Analogue { ids: &[b"TBPM"], name: "bpm", transform: plain_transform },
    Analogue { ids: &[b"TDLY"], name: "playlist_delay", transform: plain_transform },
    Analogue { ids: &[b"TLEN"], name: "length", transform: plain_transform },
    Analogue { ids: &[b"TPOS"], name: "disc", transform: plain_transform },
    Analogue { ids: &[b"TRCK"], name: "track", transform: plain_transform },
    Analogue { ids: &[b"MVIN"], name: "movement_no", transform: plain_transform },
    Analogue { ids: &[b"TDEN"], name: "encoding_date", transform: plain_transform }, // [ID3v2.4]
    Analogue { ids: &[b"TDOR"], name: "original_release_date", transform: plain_transform }, // [ID3v2.4]
    Analogue { ids: &[b"TDRL"], name: "release_date", transform: plain_transform }, // [ID3v2.4]
    Analogue { ids: &[b"TDTG"], name: "tagging_date", transform: plain_transform }, // [ID3v2.4]
    Analogue { ids: &[b"TIPL", b"IPLS"], name: "people", transform: plain_transform },
    Analogue { ids: &[b"TMCL"], name: "musicians", transform: plain_transform }, // [ID3v2.4]
    Analogue { ids: &[b"WCOM"], name: "product_url", transform: plain_transform },
    Analogue { ids: &[b"WCOP"], name: "copyright_url", transform: plain_transform },
    Analogue { ids: &[b"WOAF"], name: "file_url", transform: plain_transform },
    Analogue { ids: &[b"WOAR"], name: "artist_url", transform: plain_transform },
    Analogue { ids: &[b"WOAS"], name: "source_url", transform: plain_transform },
    Analogue { ids: &[b"WORS"], name: "station_url", transform: plain_transform },
    Analogue { ids: &[b"WPAY"], name: "payment_url", transform: plain_transform },
    Analogue { ids: &[b"WPUB"], name: "publisher_url", transform: plain_transform },
    Analogue { ids: &[b"APIC"], name: "picture", transform: plain_transform },
    Analogue { ids: &[b"TDRC", b"TYER", b"TDAT", b"TIME"], name: "date", transform: date_transform },
    Analogue { ids: &[b"COMM"], name: "comment", transform: comm_transform },
    Analogue { ids: &[b"TXXX"], name: "custom_text", transform: txxx_transform },
    Analogue { ids: &[b"WXXX"], name: "custom_url", transform: wxxx_transform },
    Analogue { ids: &[b"CHAP"], name: "chapter", transform: plain_transform },
    Analogue { ids: &[b"CTOC"], name: "table_of_contents", transform: plain_transform },
];

// Basic frame transformation using the name and
// the string representation of the frame.
fn plain_transform(name: &'static str, frame: &dyn Frame) -> DisplayTag {
    DisplayTag {
        name: DisplayName::Name(name),
        value: frame.to_string(),
    }
}

// COMM frame transformation, adding the description alongside the normal name.
fn comm_transform(name: &'static str, frame: &dyn Frame) -> DisplayTag {
    let comm = frame.downcast::<CommentsFrame>().unwrap();

    let name = if comm.desc.is_empty() {
        DisplayName::Name(name)
    } else {
        DisplayName::Custom(name, format!["{} ({})", name, comm.desc])
    };

    DisplayTag {
        name,
        value: comm.text.clone(),
    }
}

// Date frame [TDRC, TYER, TDAT, TIME] transformation, which adds proper
// clarification to frame names while still aliasing all of them under "date".
// TODO: Don't really like this, considering merging all of these into a
//  single display tag for consistency with other tag types.
fn date_transform(name: &'static str, frame: &dyn Frame) -> DisplayTag {
    let name = match frame.id().as_ref() {
        b"TDRC" => DisplayName::Name(name),
        b"TYER" => DisplayName::Custom(name, String::from("year")),
        b"TDAT" => DisplayName::Custom(name, String::from("recording_date")),
        b"TIME" => DisplayName::Custom(name, String::from("recording_time")),
        _ => unreachable!(),
    };

    DisplayTag {
        name,
        value: frame.to_string(),
    }
}

// TXXX frame transformation, adding the description alongside the normal name.
fn txxx_transform(name: &'static str, frame: &dyn Frame) -> DisplayTag {
    let txxx = frame.downcast::<UserTextFrame>().unwrap();

    DisplayTag {
        name: DisplayName::Custom(name, txxx.desc.clone()),
        value: txxx.to_string(),
    }
}

// WXXX frame transformation, adding the description alongside the normal name.
fn wxxx_transform(name: &'static str, frame: &dyn Frame) -> DisplayTag {
    let wxxx = frame.downcast::<UserUrlFrame>().unwrap();

    DisplayTag {
        name: DisplayName::Custom(name, wxxx.desc.clone()),
        value: wxxx.to_string(),
    }
}
