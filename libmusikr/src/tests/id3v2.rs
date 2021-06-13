use crate::id3v2::{TagHeader, ExtendedHeader};

#[test]
fn parse_header() {
    parse_v3_header();
    parse_v4_header();
}

#[test]
fn parse_v3_header() {
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
fn parse_v4_header() {
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

#[test]
fn parse_extended_header() {
    parse_v3_extended_header();
    parse_v4_extended_header();
}

#[test]
fn parse_v3_extended_header() {
    let data = b"\x00\x00\x00\x06\x16\x16\x16\x16\x16\x16";
    let header = ExtendedHeader::parse(3, &data[..]).unwrap();

    assert_eq!(header.size, 6);
    assert_eq!(header.data, vec![0x16; 6]);
}

#[test]
fn parse_v4_extended_header() {
    let data = b"\x00\x00\x00\x0A\x01\x16\x16\x16\x16\x16";
    let header = ExtendedHeader::parse(4, &data[..]).unwrap();

    assert_eq!(header.size, 10);
    assert_eq!(header.data, vec![0x01, 0x16, 0x16, 0x16, 0x16, 0x16]);
}