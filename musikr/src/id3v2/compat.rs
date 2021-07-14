use crate::id3v2::FrameMap;
use crate::id3v2::frames::{Frame, FrameId, TextFrame, CreditsFrame, ChapterFrame, TableOfContentsFrame};
use crate::id3v2::{ParseError, ParseResult};
use log::info;

const V2_V3_CONV: &[(&[u8; 3], &[u8; 4])] = &[
    (b"BUF", b"RBUF"), // Recommended buffer size
    (b"CNT", b"PCNT"), // Play counter
    (b"COM", b"COMM"), // Comment
    (b"CRA", b"AENC"), // Audio Encryption
    // CRM has no analogue
    (b"ETC", b"ETCO"), // Event timing codes
    (b"EQU", b"EQUA"), // Equalization
    (b"GEO", b"GEOB"), // General object
    (b"IPL", b"IPLS"), // Involved people list
    (b"LNK", b"LINK"), // Linked frame
    (b"MCI", b"MCDI"), // Music CD identifier
    (b"MLL", b"MLLT"), // MPEG lookup table
    // PIC is handled separately
    (b"POP", b"POPM"), // Popularimeter
    (b"REV", b"RVRB"), // Reverb
    (b"RVA", b"RVAD"), // Relative volume adjustment
    (b"SLT", b"SYLT"), // Synced lyrics/text
    (b"STC", b"SYTC"), // Synced tempo codes
    (b"TAL", b"TALB"), // Album/Movie/Show title
    (b"TBP", b"TBPM"), // BPM
    (b"TCM", b"TCOM"), // Composer
    (b"TCO", b"TCON"), // Content type
    (b"TCR", b"TCOP"), // Copyright message
    (b"TDA", b"TDAT"), // Date
    (b"TDY", b"TDLY"), // Playlist delay
    (b"TFT", b"TFLT"), // File type
    (b"TEN", b"TENC"), // Encoded by
    (b"TIM", b"TIME"), // Recording time
    (b"TKE", b"TKEY"), // Initial key
    (b"TLA", b"TLAN"), // Language(s)
    (b"TLE", b"TLEN"), // Length
    (b"TMT", b"TMED"), // Media type
    (b"TOA", b"TOPE"), // Original artist(s)/performer(s)
    (b"TOF", b"TOFN"), // Original filename
    (b"TOL", b"TOLY"), // Original Lyricist(s)/text writer(s)
    (b"TOR", b"TORY"), // Original release year
    (b"TOT", b"TOAR"), // Original album/movie/show title
    (b"TP1", b"TPE1"), // Lead artist(s)/Lead performer(s)/Soloist(s)/Performing group
    (b"TP2", b"TPE2"), // Band/Orchestra/Accompaniment
    (b"TP3", b"TPE3"), // Conductor/Performer refinement
    (b"TP4", b"TPE4"), // Interpreted, remixed, or otherwise modified by
    (b"TPA", b"TPOS"), // Part of a set
    (b"TPB", b"TPUB"), // Publisher
    (b"TRC", b"TSRC"), // ISRC
    (b"TRD", b"TRDA"), // Recording dates
    (b"TRK", b"TRCK"), // Track
    (b"TSI", b"TSIZ"), // Size
    (b"TSS", b"TSSE"), // Software/hardware and settings used for encoding
    (b"TT1", b"TIT1"), // Content group description
    (b"TT2", b"TIT2"), // Title/Song name/Content description
    (b"TT3", b"TIT3"), // Subtitle/Description refinement
    (b"TXT", b"TEXT"), // Lyricist/text writer
    (b"TXX", b"TXXX"), // User-defined text
    (b"TYE", b"TYER"), // Year
    (b"UFI", b"UFID"), // Unique file identifier
    (b"ULT", b"USLT"), // Unsynced lyrics/text
    (b"WAF", b"WOAF"), // Official audio file webpage
    (b"WAR", b"WOAR"), // Official artist/performer webpage
    (b"WAS", b"WOAS"), // Official audio source webpage
    (b"WCM", b"WCOM"), // Commercial information
    (b"WCP", b"WCOP"), // Copyright information
    (b"WPB", b"WPUB"), // Publishers official webpage
    (b"WXX", b"WXXX"), // Publishers official webpage
    // iTunes proprietary frames
    (b"PCS", b"PCST"),
    (b"TCT", b"TCAT"),
    (b"TDR", b"TDRL"),
    (b"TDS", b"TDES"),
    (b"TID", b"TGID"),
    (b"WFD", b"WFED"),
    (b"MVN", b"MVNM"),
    (b"MVI", b"MVIN"),
    (b"GP1", b"GRP1"),
];

pub fn upgrade_v2_id(id: &[u8; 3]) -> ParseResult<FrameId> {
    // Walk the list of pairs until an ID matches
    for (v2_id, v3_id) in V2_V3_CONV {
        if *v2_id == id {
            return Ok(FrameId::new(v3_id));
        }
    }

    Err(ParseError::NotFound)
}

pub fn to_v4(frames: &mut FrameMap) {
    // The current status of frame upgrading is as follows:
    // EQUA -> Dropped [no sane conversion]
    // RVAD -> Dropped [no sane conversion]
    // TRDA -> Dropped [no sane conversion]
    // TSIZ -> Dropped [no analogue]
    // IPLS -> TIPL
    // TYER -> TRDC: [yyyy]- MM-dd  THH:mm :ss
    // TDAT -> TDRC:  yyyy -[MM-dd] THH:mm :ss
    // TIME -> TDRC:  yyyy - MM-dd [THH:mm]:ss
    // TORY -> TDOR: [yyyy]- MM-dd  THH:mm :ss

    // Convert time frames into a single TDRC frame.
    let timestamp = to_timestamp(frames);

    if !timestamp.is_empty() {
        info!("spliced timestamp {} into TDRC", timestamp);

        frames.add(Box::new(crate::text_frame! {
            b"TDRC"; timestamp
        }))
    }

    // We don't need to do any timestamp magic for TORY, just pop it off
    // and re-add it with a different name.
    if let Some(mut frame) = frames.remove("TORY") {
        info!("upgrading TORY to TDOR");

        let tory = frame.downcast_mut::<TextFrame>().unwrap();
        *tory.id_mut() = FrameId::new(b"TDOR");
        frames.add(frame);
    }

    // Like TORY, also pop off IPLS and re-add it with a new name.
    if let Some(mut frame) = frames.remove("IPLS") {
        info!("upgrading IPLS to TIPL");

        let ipls = frame.downcast_mut::<CreditsFrame>().unwrap();
        *ipls.id_mut() = FrameId::new(b"TIPL");
        frames.add(frame);
    }

    // Clear out all the frames that can't be upgraded.
    const DROPPED: &[&[u8; 4]] = &[b"EQUA", b"RVAD", b"TSIZ", b"TRDA"];

    frames.retain(|_, frame| {
        if DROPPED.contains(&frame.id().inner()) {
            info!("dropping ID3v2.4-incompatible frame {}", frame.id());
            false
        } else {
            true
        }
    });

    // Recurse into CHAP/CTOC.

    for frame in frames.get_all_mut("CHAP") {
        let chap = frame.downcast_mut::<ChapterFrame>().unwrap();
        to_v4(&mut chap.frames);
    }

    for frame in frames.get_all_mut("CTOC") {
        let ctoc = frame.downcast_mut::<TableOfContentsFrame>().unwrap();
        to_v4(&mut ctoc.frames);
    }
}

pub fn to_v3(frames: &mut FrameMap) {
    // The current status of frame downgrading is as follows:
    // EQU2 -> Dropped [no sane conversion]
    // RVA2 -> Dropped [no sane conversion]
    // ASPI -> Dropped [no analogue]
    // SEEK -> Dropped [no analogue]
    // SIGN -> Dropped [no analogue]
    // TDEN -> Dropped [no analogue]
    // TDRL -> Dropped [no analogue]
    // TDTG -> Dropped [no analogue]
    // TMOO -> Dropped [no analogue]
    // TPRO -> Dropped [no analogue]
    // TSST -> Dropped [no analogue]
    // 
    // iTunes writes these frames to ID3v2.3 tags, but we don't care.
    // TSOA -> Dropped [no analogue]
    // TSOP -> Dropped [no analogue]
    // TSOT -> Dropped [no analogue]
    // 
    // TDOR -> TORY
    // TIPL -> IPLS
    // TMCL -> IPLS
    // TRDC -> yyyy -MM-dd THH:mm :ss
    //         TYER  TDAT   TIME

    // Turn TDOR back into TORY.
    if let Some(frame) = frames.remove("TDOR") {
        info!("downgrading TDOR to TORY");

        let tory: &TextFrame = frame.downcast().unwrap();

        if !tory.is_empty() {
            let year = tory.text[0].splitn(2, |ch: char| !ch.is_ascii_digit()).next().unwrap();

            frames.add(Box::new(crate::text_frame! {
                b"TORY"; year
            }));
        }
    }

    // Merge TIPL and TMCL into an IPLS frame. For efficiency, we will just change the ID
    // of one frame and then merge it with another frame if its present.
    match (frames.remove("TIPL"), frames.remove("TMCL")) {
        (Some(mut tipl_frame), Some(tmcl_frame))  => {
            info!("merging TIPL and TMCL into IPLS");

            let tipl = tipl_frame.downcast_mut::<CreditsFrame>().unwrap();
            let tmcl = tmcl_frame.downcast::<CreditsFrame>().unwrap();

            *tipl.id_mut() = FrameId::new(b"IPLS");
            tipl.people.extend(tmcl.people.clone());

            frames.add(tipl_frame);
        },
        (Some(mut tipl_frame), None) => {
            info!("downgrading TIPL into IPLS");

            let tipl = tipl_frame.downcast_mut::<CreditsFrame>().unwrap();
            *tipl.id_mut() = FrameId::new(b"IPLS");

            frames.add(tipl_frame);
        },
        (None, Some(mut tmcl_frame)) => {
            info!("downgrading TMCL into IPLS");

            let tmcl = tmcl_frame.downcast_mut::<CreditsFrame>().unwrap();
            *tmcl.id_mut() = FrameId::new(b"IPLS");
            frames.add(tmcl_frame);
        },
        (None, None) => {}
    }

    // Convert the TDRC frame into it's ID3v2.3 counterparts.
    from_timestamp(frames);

    // Finally drop the remaining frames with no analogue.
    const DROPPED: &[&[u8; 4]] = &[
        b"EQU2", b"RVA2", b"ASPI", b"SEEK",
        b"SIGN", b"TDEN", b"TDRL", b"TDTG",
        b"TMOO", b"TPRO", b"TSST", b"TSOA",
        b"TSOP", b"TSOT"
    ];

    frames.retain(|_, frame| {
        if DROPPED.contains(&frame.id().inner()) {
            info!("dropping ID3v2.3-incompatible frame {}", frame.id());
            false
        } else {
            true
        }
    });

    // Recurse into CHAP/CTOC.

    for frame in frames.get_all_mut("CHAP") {
        let chap = frame.downcast_mut::<ChapterFrame>().unwrap();
        to_v3(&mut chap.frames);
    }

    for frame in frames.get_all_mut("CTOC") {
        let ctoc = frame.downcast_mut::<TableOfContentsFrame>().unwrap();
        to_v3(&mut ctoc.frames);
    }
}

fn to_timestamp(frames: &mut FrameMap) -> String {
    // Turning the many ID3v2.3 date frames into TDRC mostly involves splicing
    // the required fields into the unified "yyyy-MM-ddTHH:mm:ss" timestamp.
    // Since every spliced frame builds off of the previous, the moment something
    // can't be parsed we just return the timestamp as-is.
    // Sure, *technically* regex could be used here, but bringing in the entirety
    // of that crate for just this is stupid.

    let mut timestamp = String::new();
    let tyer_frame = frames.remove("TYER");
    let tdat_frame = frames.remove("TDAT");
    let time_frame = frames.remove("TIME");

    if let Some(frame) = tyer_frame {
        // First parse the year. This can be done pretty easily by finding the last instance
        // of a year-like thing at the end of a the frame's string.
        let tyer: &TextFrame = frame.downcast().unwrap();

        if tyer.is_empty() {
            return timestamp;
        }

        let year = tyer.text[0].rsplitn(2, |ch: char| !ch.is_ascii_digit()).last().unwrap();

        if year.is_empty() {
            return timestamp;
        }

        timestamp.push_str(year);

        if let Some(frame) = tdat_frame {
            // TDAT isn't so easy. We have to find the first 4-char sequence of digits and then
            // parse that.
            let tdat: &TextFrame = frame.downcast().unwrap();

            if tdat.is_empty() {
                return timestamp;
            }

            match parse_date_pair(&tdat.text[0], '-', '-') {
                Some(date) => timestamp.push_str(&date),
                None => return timestamp
            };

            if let Some(frame) = time_frame {
                // TIME is parsed similarly to TDAT.
                let time: &TextFrame = frame.downcast().unwrap();

                if time.is_empty() {
                    return timestamp;
                }

                match parse_date_pair(&time.text[0], 'T', ':') {
                    Some(time) => timestamp.push_str(&time),
                    None => return timestamp
                };
            }
        }
    }

    timestamp
}

fn from_timestamp(frames: &mut FrameMap) {
    if let Some(frame) = frames.remove("TDRC") {
        let tdrc: &TextFrame = frame.downcast().unwrap();

        if tdrc.is_empty() {
            return
        }

        // We just split the text based on the points where are no ASCII digits.
        // This does technically open up the door for a timestamp being parsed based
        // on any non-digit character instead of just the -/T/: characters, but its
        // also the most efficient method that protects against blatantly malformed TDRC frames.
        // TODO: This is seriously busted. Find a better way to do this or create some invariant-conforming
        // abstraction.

        let mut split = tdrc.text[0].splitn(6, |ch: char| !ch.is_ascii_digit());

        match split.next() {
            Some(year) if !year.is_empty() => {
                frames.add(Box::new(crate::text_frame! {
                    b"TYER"; year
                }))
            },

            _ => return
        };

        match (split.next(), split.next()) {
            (Some(mm), Some(dd)) if mm.len() == 2 && dd.len() == 2 => {
                frames.add(Box::new(crate::text_frame! {
                    b"TDAT"; format!["{}{}", mm, dd]
                }))
            }

            _ => return
        }

        match (split.next(), split.next()) {
            (Some(hh), Some(mm)) if hh.len() == 2 && mm.len() == 2 => {
                frames.add(Box::new(crate::text_frame! {
                    b"TIME"; format!["{}{}", hh, mm]
                }))
            }

            _ => return
        }
    }
}

fn parse_date_pair(string: &str, start: char, mid: char) -> Option<String> {
    let mut chars = string.chars();
    let mut result = String::with_capacity(6);
    result.push(start);

    for i in 0..4  {
        match chars.next() {
            Some(ch) if ch.is_ascii_digit() => {
                result.push(ch);

                if i == 1 {
                    result.push(mid)
                }
            },

            _ => return None
        }
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::frames::{EqualizationFrame, RelativeVolumeFrame, EqualizationFrame2, RelativeVolumeFrame2};

    #[test]
    fn upgrade_v3_to_v4() {
        let mut frames = FrameMap::new();

        frames.add(Box::new(RelativeVolumeFrame::default()));
        frames.add(Box::new(EqualizationFrame::default()));

        frames.add(Box::new(crate::tipl_frame! {
            "Bassist" => "John Smith",
            "Violinist" => "Vanessa Evans"
        }));

        frames.add(Box::new(crate::text_frame!(b"TYER"; "2020")));
        frames.add(Box::new(crate::text_frame!(b"TDAT"; "1010")));
        frames.add(Box::new(crate::text_frame!(b"TIME"; "ABC1"))); // Make sure invalid date frames aren't spliced

        frames.add(Box::new(crate::text_frame!(b"TORY"; "2020")));

        frames.add(Box::new(crate::text_frame!(b"TRDA"; "July 12th", "May 14th")));
        frames.add(Box::new(crate::text_frame!(b"TSIZ"; "161616")));

        frames.add(Box::new(ChapterFrame {
            element_id: String::from("chp1"),
            frames: frames.clone(),
            ..Default::default()
        }));

        frames.add(Box::new(TableOfContentsFrame {
            element_id: String::from("toc1"),
            frames: frames.clone(),
            ..Default::default()
        }));

        to_v4(&mut frames);

        assert_v4_frames(&frames);

        // Test that we're recursing into metaframes
        let ctoc = frames["CHAP:chp1"].downcast::<ChapterFrame>().unwrap();
        let chap = frames["CTOC:toc1"].downcast::<TableOfContentsFrame>().unwrap();

        assert_v4_frames(&chap.frames);
        assert_v4_frames(&ctoc.frames);
    }

    fn assert_v4_frames(frames: &FrameMap) {
        assert!(!frames.contains_key("RVAD"));
        assert!(!frames.contains_any("RVA2"));
        assert!(!frames.contains_any("EQUA"));
        assert!(!frames.contains_key("EQU2"));

        assert!(!frames.contains_key("TRDA"));
        assert!(!frames.contains_key("TSIZ"));

        assert!(!frames.contains_key("TYER"));
        assert!(!frames.contains_key("TDAT"));
        assert!(!frames.contains_key("TIME"));
        assert!(!frames.contains_key("TORY"));
        assert!(!frames.contains_key("IPLS"));

        assert!(frames.contains_key("TDOR"));
        assert!(frames.contains_key("TIPL"));

        assert_eq!(frames["TDRC"].to_string(), "2020-10-10");
    }

    #[test]
    fn upgrade_v4_to_v3() {
        let mut frames = FrameMap::new();

        frames.add(Box::new(RelativeVolumeFrame2::default()));
        frames.add(Box::new(EqualizationFrame2::default()));

        // frames.add(Box::new(AudioSeekPointFrame::default())) // TODO
        // frames.add(Box::new(SignatureFrame::default())) // TODO
        // frames.add(Box::new(SeekFrame::default())) // TODO

        frames.add(Box::new(crate::text_frame! { b"TDEN"; "" }));
        frames.add(Box::new(crate::text_frame! { b"TDRL"; "" }));
        frames.add(Box::new(crate::text_frame! { b"TDTG"; "" }));
        frames.add(Box::new(crate::text_frame! { b"TMOO"; "" }));
        frames.add(Box::new(crate::text_frame! { b"TPRO"; "" }));
        frames.add(Box::new(crate::text_frame! { b"TSST"; "" }));
        frames.add(Box::new(crate::text_frame! { b"TSOA"; "" }));
        frames.add(Box::new(crate::text_frame! { b"TSOP"; "" }));
        frames.add(Box::new(crate::text_frame! { b"TSOT"; "" }));

        frames.add(Box::new(crate::text_frame! { b"TDOR"; "2020-10-10"}));

        frames.add(Box::new(crate::tipl_frame! {
            "Bassist" => "John Smith",
            "Violinist" => "Vanessa Evans"
        }));

        frames.add(Box::new(crate::tmcl_frame! {
            "Mixer" => "Matt Carver",
            "Producer" => "Sarah Oliver"   
        }));

        frames.add(Box::new(crate::text_frame! {
            b"TDRC"; "2020-10-10T40:40:20"
        }));

        frames.add(Box::new(ChapterFrame {
            element_id: String::from("chp1"),
            frames: frames.clone(),
            ..Default::default()
        }));

        frames.add(Box::new(TableOfContentsFrame {
            element_id: String::from("toc1"),
            frames: frames.clone(),
            ..Default::default()
        }));

        to_v3(&mut frames);

        assert_v3_frames(&frames);

        // Test that we're recursing into metaframes
        let ctoc = frames["CHAP:chp1"].downcast::<ChapterFrame>().unwrap();
        let chap = frames["CTOC:toc1"].downcast::<TableOfContentsFrame>().unwrap();

        assert_v3_frames(&chap.frames);
        assert_v3_frames(&ctoc.frames);
    }

    fn assert_v3_frames(frames: &FrameMap) {
        assert!(!frames.contains_key("RVA2"));
        assert!(!frames.contains_key("EQU2"));
        assert!(!frames.contains_key("ASPI"));
        assert!(!frames.contains_key("SIGN"));
        assert!(!frames.contains_key("SEEK"));
        assert!(!frames.contains_key("EQU2"));
        assert!(!frames.contains_key("TDEN"));
        assert!(!frames.contains_key("TDRL"));
        assert!(!frames.contains_key("TDTG"));
        assert!(!frames.contains_key("TMOO"));
        assert!(!frames.contains_key("TPRO"));
        assert!(!frames.contains_key("TSST"));
        assert!(!frames.contains_key("TSOA"));
        assert!(!frames.contains_key("TSOP"));
        assert!(!frames.contains_key("TSOT"));
        assert!(!frames.contains_key("TDOR"));
        assert!(!frames.contains_key("TDRC"));
        assert!(!frames.contains_key("TMCL"));

        assert_eq!(frames["TORY"].to_string(), "2020");

        let ipls = frames["TIPL"].downcast::<CreditsFrame>().unwrap();
        assert_eq!(ipls.people["Bassist"], "John Smith");
        assert_eq!(ipls.people["Violinist"], "Vanessa Evans");
        assert_eq!(ipls.people["Mixer"], "Matt Carver");
        assert_eq!(ipls.people["Producer"], "Sarah Oliver");

        assert_eq!(frames["TYER"].to_string(), "2020");
        assert_eq!(frames["TDAT"].to_string(), "1010");
        assert_eq!(frames["TIME"].to_string(), "4040");
    }
}