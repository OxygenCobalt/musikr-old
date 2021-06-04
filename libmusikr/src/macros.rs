//! Generates a u8-represented enum with a corresponding `new` function that creates an
//! enum from a given byte. The enum must implement `Default`.
macro_rules! byte_enum {(
    $(#[$meta:meta])*
    $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $variant:ident $(= $val:expr)?,)*
    }
) => {
        $(#[$meta])*
        #[repr(u8)]
        #[derive(Clone, Copy, Debug)]
        $vis enum $name {
            $($(#[$vmeta])* $variant $(= $val)?,)*
        }

        impl $name {
            pub(crate) fn new(byte: u8) -> Self {
                match byte {
                    $(byte if byte == Self::$variant as u8 => Self::$variant,)*
                    _ => Self::default()
                }
            }
        }
    }
}
