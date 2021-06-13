mod header {
    use crate::id3v2::TagHeader;

    #[test]
    fn parse_v3() {
        let data = b"\x49\x44\x33\x03\x00\xA0\x00\x08\x49\x30";
        let header = TagHeader::parse(&data[..]).unwrap();

        assert_eq!(header.tag_size, 140464);
        assert_eq!(header.major, 3);
        assert_eq!(header.minor, 0);

        assert_eq!(header.flags.unsync, true);
        assert_eq!(header.flags.extended, false);
        assert_eq!(header.flags.experimental, true)
    }

    #[test]
    fn parse_v4() {
        let data = b"\x49\x44\x33\x04\x00\x50\x00\x08\x49\x30";
        let header = TagHeader::parse(&data[..]).unwrap();

        assert_eq!(header.tag_size, 140464);
        assert_eq!(header.major, 4);
        assert_eq!(header.minor, 0);

        assert_eq!(header.flags.unsync, false);
        assert_eq!(header.flags.extended, true);
        assert_eq!(header.flags.experimental, false);
        assert_eq!(header.flags.footer, true);
    }
}

mod ext_header {
    use crate::id3v2::ExtendedHeader;

    #[test]
    fn parse_v3() {
        let data = b"\x00\x00\x00\x06\x16\x16\x16\x16\x16\x16";
        let header = ExtendedHeader::parse(3, &data[..]).unwrap();

        // Since we don't parse these headers, we just assert their size
        // and internal data is okay.
        assert_eq!(header.size, 6);
        assert_eq!(header.data, vec![0x16; 6]);
    }

    #[test]
    fn parse_v4() {
        let data = b"\x00\x00\x00\x0A\x01\x16\x16\x16\x16\x16";
        let header = ExtendedHeader::parse(4, &data[..]).unwrap();

        assert_eq!(header.size, 10);
        assert_eq!(header.data, vec![0x01, 0x16, 0x16, 0x16, 0x16, 0x16]);
    }
}
