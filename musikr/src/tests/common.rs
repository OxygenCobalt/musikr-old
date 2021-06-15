mod raw {
    use crate::raw;

    #[test]
    pub fn parse_u32() {
        let data = vec![0xAB, 0xCD, 0xDE, 0xF0];

        assert_eq!(raw::to_u32(&data), 0xABCDDEF0);
    }
    
    #[test]
    pub fn parse_u16() {
        let data = vec![0xAB, 0xCD];

        assert_eq!(raw::to_u16(&data), 0xABCD);
    }
    
    #[test]
    pub fn parse_bit() {
        let data = 0b10101101;

        assert_eq!(raw::bit_at(0, data), true);
        assert_eq!(raw::bit_at(1, data), false);
        assert_eq!(raw::bit_at(2, data), true);
        assert_eq!(raw::bit_at(3, data), true);
        assert_eq!(raw::bit_at(4, data), false);
        assert_eq!(raw::bit_at(5, data), true);
        assert_eq!(raw::bit_at(6, data), false);
        assert_eq!(raw::bit_at(7, data), true);
    }    
}
