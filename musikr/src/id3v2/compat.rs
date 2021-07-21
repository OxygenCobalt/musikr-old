use crate::id3v2::frames::{
    ChapterFrame, CreditsFrame, Frame, FrameId, NumericFrame, TableOfContentsFrame, TextFrame,
};
use crate::id3v2::{FrameMap, ParseError, ParseResult};
use log::info;
use std::str::Chars;

static V2_V3_CONV: &[(&[u8; 3], &[u8; 4])] = &[
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

static V3_UNSUPPORTED: &[&[u8; 4]] = &[
    b"EQU2", b"RVA2", b"ASPI", b"SEEK", b"SIGN", b"TDEN", b"TDRL", b"TDTG", b"TMOO", b"TPRO",
    b"TSST", b"TSOA", b"TSOP", b"TSOT",
];

static V4_UNSUPPORTED: &[&[u8; 4]] = &[b"EQUA", b"RVAD", b"TSIZ", b"TRDA"];

pub fn upgrade_v2_id(id: &[u8; 3]) -> ParseResult<FrameId> {
    // Walk the list of pairs until an ID matches
    for (v2_id, v3_id) in V2_V3_CONV {
        if *v2_id == id {
            return Ok(FrameId::new(v3_id));
        }
    }

    Err(ParseError::NotFound)
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

    // Convert the TDRC frame into it's ID3v2.3 counterparts.
    if let Some(frame) = frames.remove("TDRC") {
        from_tdrc(frame.downcast::<TextFrame>().unwrap(), frames)
    }

    // Turn TDOR back into TORY. It's a bit more difficult here since we have to deal with
    // the timestamp.
    if let Some(frame) = frames.remove("TDOR") {
        let tdor = frame.downcast::<TextFrame>().unwrap();
        let mut tory = NumericFrame::new(FrameId::new(b"TORY"));

        for timestamp in &tdor.text {
            if let Some(yyyy) = parse_timestamp(&mut timestamp.chars(), '-') {
                // Like find_year, tolerate years that aren't four chars.
                tory.text.push(format!["{:0>4}", yyyy].parse().unwrap())
            }
        }

        info!("downgraded TDOR to TORY: {}", tory);

        frames.add(tory)
    }

    // Merge TIPL and TMCL into an IPLS frame. For efficiency, we will just change the ID
    // of one frame and then merge it with another frame if its present.
    match (frames.remove("TIPL"), frames.remove("TMCL")) {
        (Some(mut tipl_frame), Some(tmcl_frame)) => {
            info!("merging TIPL and TMCL into IPLS");

            let tipl = tipl_frame.downcast_mut::<CreditsFrame>().unwrap();
            let tmcl = tmcl_frame.downcast::<CreditsFrame>().unwrap();

            *tipl.id_mut() = FrameId::new(b"IPLS");
            tipl.people.extend(tmcl.people.clone());

            frames.add_boxed(tipl_frame);
        }
        (Some(mut tipl_frame), None) => {
            info!("downgrading TIPL into IPLS");

            let tipl = tipl_frame.downcast_mut::<CreditsFrame>().unwrap();
            *tipl.id_mut() = FrameId::new(b"IPLS");

            frames.add_boxed(tipl_frame);
        }
        (None, Some(mut tmcl_frame)) => {
            info!("downgrading TMCL into IPLS");

            let tmcl = tmcl_frame.downcast_mut::<CreditsFrame>().unwrap();
            *tmcl.id_mut() = FrameId::new(b"IPLS");
            frames.add_boxed(tmcl_frame);
        }
        (None, None) => {}
    }

    // Drop the remaining frames with no analogue.
    frames.retain(|_, frame| {
        if V3_UNSUPPORTED.contains(&frame.id().inner()) {
            info!("dropping ID3v2.3-incompatible frame {}", frame.id());
            false
        } else {
            true
        }
    });

    // Recurse into CHAP/CTOC.

    for frame in frames.get_all_mut(b"CHAP") {
        let chap = frame.downcast_mut::<ChapterFrame>().unwrap();
        to_v3(&mut chap.frames);
    }

    for frame in frames.get_all_mut(b"CTOC") {
        let ctoc = frame.downcast_mut::<TableOfContentsFrame>().unwrap();
        to_v3(&mut ctoc.frames);
    }
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
    let tdrc = to_tdrc(frames);

    if !tdrc.is_empty() {
        info!("upgraded to TDRC: {}", tdrc);

        frames.add(tdrc);
    }

    // We don't need to do any timestamp magic for TORY, just pop it off
    // and re-add it with a different name.
    if let Some(frame) = frames.remove("TORY") {
        let tory = frame.downcast::<NumericFrame>().unwrap();
        let mut tdor = TextFrame::new(FrameId::new(b"TDOR"));

        for year in &tory.text {
            tdor.text.push(format!["{:0>4}", year])
        }

        info!("upgraded TORY to TDOR: {}", tdor);

        frames.add(tdor)
    }

    // Like TORY, also pop off IPLS and re-add it with a new name.
    if let Some(mut frame) = frames.remove("IPLS") {
        info!("upgrading IPLS to TIPL");

        let ipls = frame.downcast_mut::<CreditsFrame>().unwrap();
        *ipls.id_mut() = FrameId::new(b"TIPL");
        frames.add_boxed(frame);
    }

    // Clear out all the frames that can't be upgraded.
    frames.retain(|_, frame| {
        if V4_UNSUPPORTED.contains(&frame.id().inner()) {
            info!("dropping ID3v2.4-incompatible frame {}", frame.id());
            false
        } else {
            true
        }
    });

    // Recurse into CHAP/CTOC.

    for frame in frames.get_all_mut(b"CHAP") {
        let chap = frame.downcast_mut::<ChapterFrame>().unwrap();
        to_v4(&mut chap.frames);
    }

    for frame in frames.get_all_mut(b"CTOC") {
        let ctoc = frame.downcast_mut::<TableOfContentsFrame>().unwrap();
        to_v4(&mut ctoc.frames);
    }
}

fn to_tdrc(frames: &mut FrameMap) -> TextFrame {
    // Turning the many ID3v2.3 date frames into TDRC mostly involves splicing
    // the required fields into the unified "yyyy-MM-ddTHH:mm:ss" timestamp.
    // Parsing this is actually quite annoying, since it's impossible to assume
    // that TYER/TDAT/TIME are actually sane, but we try our best.

    let tyer_frame = frames.remove("TYER");
    let tdat_frame = frames.remove("TDAT");
    let time_frame = frames.remove("TIME");

    // Like all text frames, TYER/TDAT/TIME can also contain multiple values. As a result, we keep iterators
    // for all the strings in these frames and zip them together into a timestamp as we go along.
    let mut tyer = match tyer_frame {
        Some(ref frame) => frame.downcast::<NumericFrame>().unwrap().text.iter(),
        None => [].iter(),
    };

    let mut tdat = match tdat_frame {
        Some(ref frame) => frame.downcast::<NumericFrame>().unwrap().text.iter(),
        None => [].iter(),
    };

    let mut time = match time_frame {
        Some(ref frame) => frame.downcast::<NumericFrame>().unwrap().text.iter(),
        None => [].iter(),
    };

    let mut timestamps = Vec::new();

    // YYYY strings have no defined limit in size, but MMHH/HHMM strings must be 4 characters.

    loop {
        let mut timestamp = String::new();

        match tyer.next() {
            Some(yyyy) => {
                timestamp.push_str(&format!["{:0>4}", yyyy]);
            }

            // Timestamps are now exhausted, exit the loop.
            _ => break,
        }

        if let Some(mmdd) = tdat.next() {
            if mmdd.len() >= 4 {
                timestamp.push_str(&format!["-{}-{}", &mmdd[0..2], &mmdd[2..4]]);

                if let Some(hhmm) = time.next() {
                    if hhmm.len() >= 4 {
                        timestamp.push_str(&format!["T{}:{}", &hhmm[0..2], &hhmm[2..4]]);
                    }
                }
            }
        }

        if !timestamp.is_empty() {
            timestamps.push(timestamp)
        }
    }

    let mut tdrc = TextFrame::new(FrameId::new(b"TDRC"));
    tdrc.text = timestamps;

    tdrc
}

fn from_tdrc(tdrc: &TextFrame, frames: &mut FrameMap) {
    let mut tyer = NumericFrame::new(FrameId::new(b"TYER"));
    let mut tdat = NumericFrame::new(FrameId::new(b"TDAT"));
    let mut time = NumericFrame::new(FrameId::new(b"TIME"));

    // Detect a valid timestamp. This is quite strict, but prevents weird timestamps from
    // causing malformed data.
    for stamp in &tdrc.text {
        let mut chars = stamp.chars();

        // We walk until we hit either:
        // - A stopping character [-/T/:]
        // - The end of the iterator
        // - A non-digit character, which causes parsing to fail.

        // Like find_year, tolerate years that aren't four chars.
        match parse_timestamp(&mut chars, '-') {
            Some(yyyy) if !yyyy.is_empty() => {
                tyer.text.push(format!["{:0>4}", yyyy].parse().unwrap())
            }
            _ => continue,
        }

        match (
            parse_timestamp(&mut chars, '-'),
            parse_timestamp(&mut chars, 'T'),
        ) {
            (Some(mm), Some(dd)) if mm.len() == 2 && dd.len() == 2 => {
                tdat.text.push(format!["{}{}", dd, mm].parse().unwrap())
            }

            _ => continue,
        }

        match (
            parse_timestamp(&mut chars, ':'),
            parse_timestamp(&mut chars, ':'),
        ) {
            (Some(hh), Some(mm)) if hh.len() == 2 && mm.len() == 2 => {
                time.text.push(format!["{}{}", hh, mm].parse().unwrap())
            }

            _ => continue,
        }
    }

    if !tyer.is_empty() {
        frames.add(tyer)
    }

    if !tdat.is_empty() {
        frames.add(tdat)
    }

    if !time.is_empty() {
        frames.add(time)
    }
}

fn parse_timestamp(chars: &mut Chars, sep: char) -> Option<String> {
    let mut string = String::new();

    loop {
        match chars.next() {
            Some(ch) if ch.is_ascii_digit() => string.push(ch),
            Some(ch) if ch == sep => break,
            Some(_) => return None,
            None => break,
        }
    }

    Some(string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::frames::{
        EqualizationFrame, EqualizationFrame2, RelativeVolumeFrame, RelativeVolumeFrame2,
    };

    #[test]
    fn upgrade_v3_to_v4() {
        let mut frames = FrameMap::new();

        frames.add(RelativeVolumeFrame::default());
        frames.add(EqualizationFrame::default());

        frames.add(crate::credits_frame! {
            b"TMCL",
            "Bassist" => "John Smith",
            "Violinist" => "Vanessa Evans"
        });

        frames.add(crate::numeric_frame!(b"TYER", "2020"));
        frames.add(crate::numeric_frame!(b"TDAT", "1010"));
        frames.add(crate::numeric_frame!(b"TIME", "12"));

        frames.add(crate::numeric_frame!(b"TORY", "2020"));

        frames.add(crate::text_frame!(b"TRDA"; "July 12th", "May 14th"));
        frames.add(crate::numeric_frame!(b"TSIZ", "161616"));

        frames.add(ChapterFrame {
            element_id: String::from("chp1"),
            frames: frames.clone(),
            ..Default::default()
        });

        frames.add(TableOfContentsFrame {
            element_id: String::from("toc1"),
            frames: frames.clone(),
            ..Default::default()
        });

        to_v4(&mut frames);

        assert_v4_frames(&frames);

        // Test that we're recursing into metaframes
        let ctoc = frames["CHAP:chp1"].downcast::<ChapterFrame>().unwrap();
        let chap = frames["CTOC:toc1"]
            .downcast::<TableOfContentsFrame>()
            .unwrap();

        assert_v4_frames(&chap.frames);
        assert_v4_frames(&ctoc.frames);
    }

    fn assert_v4_frames(frames: &FrameMap) {
        assert!(!frames.contains_key("RVAD"));
        assert!(!frames.contains_any(b"RVA2"));
        assert!(!frames.contains_any(b"EQUA"));
        assert!(!frames.contains_key("EQU2"));

        assert!(!frames.contains_key("TRDA"));
        assert!(!frames.contains_key("TSIZ"));

        assert!(!frames.contains_key("TYER"));
        assert!(!frames.contains_key("TDAT"));
        assert!(!frames.contains_key("TIME"));
        assert!(!frames.contains_key("TORY"));
        assert!(!frames.contains_key("IPLS"));

        assert!(frames.contains_key("TDOR"));
        assert!(frames.contains_key("TMCL"));

        assert_eq!(frames["TDRC"].to_string(), "2020-10-10");
    }

    #[test]
    fn upgrade_v4_to_v3() {
        const FULL: &str = "2020-01-01T12:34:00";
        const NO_SEC: &str = "2021-02-02T16:16";
        const NO_MIN: &str = "2022-03-03T32";
        const NO_TIME: &str = "2023-04-04";
        const NO_DAY: &str = "2024-05";
        const NO_MONTH: &str = "2025";

        let mut frames = FrameMap::new();

        frames.add(RelativeVolumeFrame2::default());
        frames.add(EqualizationFrame2::default());

        // frames.add(AudioSeekPointFrame::default())) // TODO
        // frames.add(SignatureFrame::default())) // TODO
        // frames.add(SeekFrame::default())) // TODO

        frames.add(crate::text_frame!(b"TDEN"));
        frames.add(crate::text_frame!(b"TDRL"));
        frames.add(crate::text_frame!(b"TDTG"));
        frames.add(crate::text_frame!(b"TMOO"));
        frames.add(crate::text_frame!(b"TPRO"));
        frames.add(crate::text_frame!(b"TSST"));
        frames.add(crate::text_frame!(b"TSOA"));
        frames.add(crate::text_frame!(b"TSOP"));
        frames.add(crate::text_frame!(b"TSOT"));

        frames.add(crate::text_frame! {
            b"TDOR"; "2020-10-10T40:40"
        });

        frames.add(crate::credits_frame! {
            b"TMCL",
            "Bassist" => "John Smith",
            "Violinist" => "Vanessa Evans"
        });

        frames.add(crate::credits_frame! {
            b"TIPL",
            "Mixer" => "Matt Carver",
            "Producer" => "Sarah Oliver"
        });

        frames.add(crate::text_frame! {
            b"TDRC"; FULL, NO_SEC, NO_MIN, NO_TIME, NO_DAY, NO_MONTH
        });

        frames.add(ChapterFrame {
            element_id: String::from("chp1"),
            frames: frames.clone(),
            ..Default::default()
        });

        frames.add(TableOfContentsFrame {
            element_id: String::from("toc1"),
            frames: frames.clone(),
            ..Default::default()
        });

        to_v3(&mut frames);

        assert_v3_frames(&frames);

        // Test that we're recursing into metaframes
        let ctoc = frames["CHAP:chp1"].downcast::<ChapterFrame>().unwrap();
        let chap = frames["CTOC:toc1"]
            .downcast::<TableOfContentsFrame>()
            .unwrap();

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

        assert_eq!(
            frames["TYER"].to_string(),
            "2020, 2021, 2022, 2023, 2024, 2025"
        );
        assert_eq!(frames["TDAT"].to_string(), "0101, 0202, 0303, 0404");
        assert_eq!(frames["TIME"].to_string(), "1234, 1616");
    }
}
