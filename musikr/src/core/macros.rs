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

macro_rules! inner_eq {
    ($lhs:ty, $rhs:ty) => {
        impl<'a, 'b> PartialEq<$rhs> for $lhs {
            fn eq(&self, other: &$rhs) -> bool {
                self.0.eq(&other[..])
            }
        }
    };
}

macro_rules! inner_borrow {
    ($typ:ty, $out:ty) => {
        impl std::borrow::Borrow<$out> for $typ {
            fn borrow(&self) -> &$out {
                &self.0[..]
            }
        }
    };
}

macro_rules! inner_index {
    ($typ:ty, $out:ty) => {
        impl std::ops::Index<usize> for $typ {
            type Output = $out;

            fn index(&self, idx: usize) -> &Self::Output {
                self.0.index(idx)
            }
        }
    };
}

macro_rules! inner_ranged_index {
    ($typ:ty, $out:ty) => {
        impl std::ops::Index<std::ops::Range<usize>> for $typ {
            type Output = $out;

            fn index(&self, idx: std::ops::Range<usize>) -> &Self::Output {
                self.0.index(idx)
            }
        }

        impl std::ops::Index<std::ops::RangeTo<usize>> for $typ {
            type Output = $out;

            #[inline]
            fn index(&self, idx: std::ops::RangeTo<usize>) -> &Self::Output {
                self.0.index(idx)
            }
        }

        impl std::ops::Index<std::ops::RangeFrom<usize>> for $typ {
            type Output = $out;

            #[inline]
            fn index(&self, idx: std::ops::RangeFrom<usize>) -> &Self::Output {
                self.0.index(idx)
            }
        }

        impl std::ops::Index<std::ops::RangeInclusive<usize>> for $typ {
            type Output = $out;

            #[inline]
            fn index(&self, idx: std::ops::RangeInclusive<usize>) -> &Self::Output {
                self.0.index(idx)
            }
        }

        impl std::ops::Index<std::ops::RangeToInclusive<usize>> for $typ {
            type Output = $out;

            #[inline]
            fn index(&self, idx: std::ops::RangeToInclusive<usize>) -> &Self::Output {
                self.0.index(idx)
            }
        }
    };
}
