/// Takes an enum definition with corresponding integer values and generates a `repr(u8)` enum
/// with a corresponding `parse` function that takes a `u8` and returns its corresponding enum
/// variant. If the byte cannot be matched, `err` is returned.
macro_rules! byte_enum {(
    $(#[$meta:meta])*
    $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $variant:ident = $val:expr,)*
    };
    $err:expr
) => {
        $(#[$meta])*
        #[repr(u8)]
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        $vis enum $name {
            $($(#[$vmeta])*
            $variant = $val,)*
        }

        impl $name {
            pub(crate) fn parse(byte: u8) -> Self {
                match byte {
                    $($val => Self::$variant,)*
                    _ => $err
                }
            }
        }
    }
}
