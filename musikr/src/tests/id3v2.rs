mod header {
    use crate::id3v2::TagHeader;

    #[test]
    fn parse_v3() {
        let data = b"\x49\x44\x33\x03\x00\xA0\x00\x08\x49\x30";
        let header = TagHeader::parse(&data[..]).unwrap();

        let tag_size = header.size();
        let version = header.version();
        let flags = header.flags();

        assert_eq!(tag_size, 140464);
        assert_eq!(version.0, 3);
        assert_eq!(version.1, 0);

        assert_eq!(flags.unsync, true);
        assert_eq!(flags.extended, false);
        assert_eq!(flags.experimental, true)
    }

    #[test]
    fn parse_v4() {
        let data = b"\x49\x44\x33\x04\x00\x50\x00\x08\x49\x30";
        let header = TagHeader::parse(&data[..]).unwrap();

        let tag_size = header.size();
        let version = header.version();
        let flags = header.flags();

        assert_eq!(tag_size, 140464);
        assert_eq!(version.0, 4);
        assert_eq!(version.1, 0);

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
        let header = ExtendedHeader::parse((3, 0), &data[..]).unwrap();

        assert_eq!(header.size(), 6);
        assert_eq!(header.data(), vec![0x16; 6]);
    }

    #[test]
    fn parse_v4() {
        let data = b"\x00\x00\x00\x0A\x01\x16\x16\x16\x16\x16";
        let header = ExtendedHeader::parse((4, 0), &data[..]).unwrap();

        assert_eq!(header.size(), 10);
        assert_eq!(header.data(), vec![0x01, 0x16, 0x16, 0x16, 0x16, 0x16]);
    }
}

mod frame_header {
    use crate::id3v2::frames::FrameHeader;

    #[test]
    fn parse_v3() {
        let data = b"TXXX\x00\x0A\x71\x7B\xA0\x40";
        let header = FrameHeader::parse((3, 0), &data[..]).unwrap();
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
        let header = FrameHeader::parse((4, 0), &data[..]).unwrap();
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
