use crate::id3v2::frames::FrameId;
use crate::id3v2::{ParseError, ParseResult};

const V2_V3_CONV: &[(&[u8; 3], &[u8; 4])] = &[
    (b"BUF", b"RBUF"), // Recommended buffer size
    (b"CNT", b"PCNT"), // Play counter
    (b"COM", b"COMM"), // Comment
    (b"CRA", b"AENC"), // Audio Encryption
    // CRM has no analogue
    (b"ETC", b"ETCO"), // Event timing codes
    (b"EQU", b"EQUA"), // Equalisation
    (b"GEO", b"GEOB"), // General object
    (b"IPL", b"IPLS"), // Involved people list
    (b"LNK", b"LINK"), // Linked frame
    (b"MCI", b"MCDI"), // Music CD identifier
    (b"MLL", b"MLLT"), // MPEG lookup table
    // PIC is handled seperately
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
    (b"TOT", b"TOAR"), // Origional album/movie/show title
    (b"TP1", b"TPE1"), // Lead artist(s)/Lead performer(s)/Soloist(s)/Performing group
    (b"TP2", b"TPE2"), // Band/Orchestra/Accompanient
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
    (b"TT2", b"TIT2"), // Title/Songname/Content description
    (b"TT3", b"TIT3"), // Subtitle/Description refinement
    (b"TXT", b"TEXT"), // Lyricist/text writer
    (b"TXX", b"TXXX"), // User-defined text
    (b"TYE", b"TYER"), // Year
    (b"UFI", b"UFID"), // Unique file identifer
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

    // No dice.
    Err(ParseError::NotFound)
}
