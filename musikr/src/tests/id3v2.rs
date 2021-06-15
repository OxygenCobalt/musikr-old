mod header {
    use crate::id3v2::TagHeader;

    #[test]
    fn parse_v3() {
        let data = b"\x49\x44\x33\x03\x00\xA0\x00\x08\x49\x30";
        let header = TagHeader::parse(&data[..]).unwrap();
        let flags = header.flags();

        assert_eq!(header.size(), 140464);
        assert_eq!(header.major(), 3);
        assert_eq!(header.minor(), 0);

        assert_eq!(flags.unsync, true);
        assert_eq!(flags.extended, false);
        assert_eq!(flags.experimental, true)
    }

    #[test]
    fn parse_v4() {
        let data = b"\x49\x44\x33\x04\x00\x50\x00\x08\x49\x30";
        let header = TagHeader::parse(&data[..]).unwrap();
        let flags = header.flags();

        assert_eq!(header.size(), 140464);
        assert_eq!(header.major(), 4);
        assert_eq!(header.minor(), 0);

        assert_eq!(flags.unsync, false);
        assert_eq!(flags.extended, true);
        assert_eq!(flags.experimental, false);
        assert_eq!(flags.footer, true);
    }
}

mod ext_header {
    use crate::id3v2::ExtendedHeader;

    #[test]
    fn parse_v3() {
        let data = b"\x00\x00\x00\x06\x16\x16\x16\x16\x16\x16";
        let header = ExtendedHeader::parse(3, &data[..]).unwrap();

        assert_eq!(header.size(), 6);
        assert_eq!(header.data(), &vec![0x16; 6]);
    }

    #[test]
    fn parse_v4() {
        let data = b"\x00\x00\x00\x0A\x01\x16\x16\x16\x16\x16";
        let header = ExtendedHeader::parse(4, &data[..]).unwrap();

        assert_eq!(header.size(), 10);
        assert_eq!(header.data(), &vec![0x01, 0x16, 0x16, 0x16, 0x16, 0x16]);
    }
}

mod frame_header {
    use crate::id3v2::frames::FrameHeader;

    #[test]
    fn parse_v3() {
        let data = b"TXXX\x00\x0A\x71\x7B\xA0\x40";
        let header = FrameHeader::parse(3, &data[..]).unwrap();
        let flags = header.flags();

        assert_eq!(header.id(), "TXXX");
        assert_eq!(header.size(), 684411);

        assert_eq!(flags.tag_should_discard, true);
        assert_eq!(flags.file_should_discard, false);
        assert_eq!(flags.read_only, true);

        assert_eq!(flags.compressed, false);
        assert_eq!(flags.encrypted, true);
        assert_eq!(flags.has_group, false);
    }

    #[test]
    fn parse_v4() {
        let data = b"TXXX\x00\x34\x10\x2A\x50\x4B";
        let header = FrameHeader::parse(4, &data[..]).unwrap();
        let flags = header.flags();

        assert_eq!(header.id(), "TXXX");
        assert_eq!(header.size(), 854058);

        assert_eq!(flags.tag_should_discard, true);
        assert_eq!(flags.file_should_discard, false);
        assert_eq!(flags.read_only, true);

        assert_eq!(flags.has_group, true);
        assert_eq!(flags.compressed, true);
        assert_eq!(flags.encrypted, false);
        assert_eq!(flags.unsync, true);
        assert_eq!(flags.has_data_len, true);
    }
}

mod string {
    use crate::id3v2::frames::string::{self, Encoding};

    #[test]
    fn parse_latin1() {
        let data = b"\x4c\xee\x6b\x65\x20\xe2\x20\x77\x68\x69\x6c\x65\x20\x6c\x6f\x6f\x70\x20\x77\xef\x74\x68\x20\x6e\xf8\x20\x65\x73\x63\x61\x70\xea";

        assert_eq!(string::get_string(Encoding::Latin1, data), "Lîke â while loop wïth nø escapê")
    }

    #[test]
    fn parse_utf16() {
        let data = b"\xFF\xFE\x51\x25\x20\x00\x4c\x00\xee\x00\x6b\x00\x65\x00\x20\x00\xe2\x00\x20\x00\x35\xd8\x68\xdd\x35\xd8\x59\xdd\x35\xd8\x5a\xdd\x6c\x00\x65\x00\x20\x00\x6c\x00\x35\xd8\x90\xdc\x35\xd8\x90\xdc\x70\x00\x20\x00\x77\x00\xef\x00\x74\x00\x68\x00\x20\x00\x6e\x00\xf8\x00\x20\x00\x65\x00\x73\x00\x63\x00\x61\x00\x70\x00\xea\x00\x20\x00\x51\x25";

        assert_eq!(string::get_string(Encoding::Utf16, data), "║ Lîke â 𝕨𝕙𝕚le l𝒐𝒐p wïth nø escapê ║")
    }

    #[test]
    fn parse_utf16be() {
        let data = b"\x25\x51\x00\x20\x00\x4c\x00\xee\x00\x6b\x00\x65\x00\x20\x00\xe2\x00\x20\xd8\x35\xdd\x68\xd8\x35\xdd\x59\xd8\x35\xdd\x5a\x00\x6c\x00\x65\x00\x20\x00\x6c\xd8\x35\xdc\x90\xd8\x35\xdc\x90\x00\x70\x00\x20\x00\x77\x00\xef\x00\x74\x00\x68\x00\x20\x00\x6e\x00\xf8\x00\x20\x00\x65\x00\x73\x00\x63\x00\x61\x00\x70\x00\xea\x00\x20\x25\x51";

        assert_eq!(string::get_string(Encoding::Utf16Be, data), "║ Lîke â 𝕨𝕙𝕚le l𝒐𝒐p wïth nø escapê ║")
    }

    #[test]
    fn parse_utf16le() {
        let data = b"\x51\x25\x20\x00\x4c\x00\xee\x00\x6b\x00\x65\x00\x20\x00\xe2\x00\x20\x00\x35\xd8\x68\xdd\x35\xd8\x59\xdd\x35\xd8\x5a\xdd\x6c\x00\x65\x00\x20\x00\x6c\x00\x35\xd8\x90\xdc\x35\xd8\x90\xdc\x70\x00\x20\x00\x77\x00\xef\x00\x74\x00\x68\x00\x20\x00\x6e\x00\xf8\x00\x20\x00\x65\x00\x73\x00\x63\x00\x61\x00\x70\x00\xea\x00\x20\x00\x51\x25";

        assert_eq!(string::get_string(Encoding::Utf16Le, data), "║ Lîke â 𝕨𝕙𝕚le l𝒐𝒐p wïth nø escapê ║")
    }

    #[test]
    fn parse_utf8() {
        let data = b"\xe2\x95\x91\x20\x4c\xc3\xae\x6b\x65\x20\xc3\xa2\x20\xf0\x9d\x95\xa8\xf0\x9d\x95\x99\xf0\x9d\x95\x9a\x6c\x65\x20\x6c\xf0\x9d\x92\x90\xf0\x9d\x92\x90\x70\x20\x77\xc3\xaf\x74\x68\x20\x6e\xc3\xb8\x20\x65\x73\x63\x61\x70\xc3\xaa\x20\xe2\x95\x91";

        assert_eq!(string::get_string(Encoding::Utf8, data), "║ Lîke â 𝕨𝕙𝕚le l𝒐𝒐p wïth nø escapê ║")
    }
}
