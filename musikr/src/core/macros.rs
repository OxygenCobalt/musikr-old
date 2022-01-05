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

macro_rules! impl_array_newtype {(
    $typ:ty, $err:ty, $n: expr
) => {
    impl<'a, 'b> PartialEq<[u8; $n]> for $typ {
        fn eq(&self, other: &[u8; $n]) -> bool {
            self.0.eq(&other[..])
        }
    }

    impl<'a, 'b> PartialEq<&[u8; $n]> for $typ {
        fn eq(&self, other: &&[u8; $n]) -> bool {
            self.0.eq(&other[..])
        }
    }

    impl<'a, 'b> PartialEq<&[u8]> for $typ {
        fn eq(&self, other: &&[u8]) -> bool {
            self.0.eq(&other[..])
        }
    }

    // TODO: Consider re-adding the ability to reference a newtype as a slice.

    impl AsRef<[u8; $n]> for $typ {
        fn as_ref(&self) -> &[u8; $n] {
            &self.0
        }
    }

    impl std::borrow::Borrow<[u8; $n]> for $typ {
        fn borrow(&self) -> &[u8; $n] {
            &self.0
        }
    }

    impl TryFrom<[u8; $n]> for $typ {
        type Error = $err;

        fn try_from(other: [u8; $n]) -> Result<Self, Self::Error> {
            Self::try_new(&other)
        }
    }

    impl TryFrom<&[u8; $n]> for $typ {
        type Error = $err;

        fn try_from(other: &[u8; $n]) -> Result<Self, Self::Error> {
            Self::try_new(&other)
        }
    }

    impl std::ops::Index<usize> for $typ {
        type Output = u8;

        fn index(&self, idx: usize) -> &Self::Output {
            self.0.index(idx)
        }
    }

    impl std::ops::Index<std::ops::Range<usize>> for $typ {
        type Output = [u8];

        fn index(&self, idx: std::ops::Range<usize>) -> &Self::Output {
            self.0.index(idx)
        }
    }

    impl std::ops::Index<std::ops::RangeTo<usize>> for $typ {
        type Output = [u8];

        #[inline]
        fn index(&self, idx: std::ops::RangeTo<usize>) -> &Self::Output {
            self.0.index(idx)
        }
    }

    impl std::ops::Index<std::ops::RangeFrom<usize>> for $typ {
        type Output = [u8];

        #[inline]
        fn index(&self, idx: std::ops::RangeFrom<usize>) -> &Self::Output {
            self.0.index(idx)
        }
    }

    impl std::ops::Index<std::ops::RangeInclusive<usize>> for $typ {
        type Output = [u8];

        #[inline]
        fn index(&self, idx: std::ops::RangeInclusive<usize>) -> &Self::Output {
            self.0.index(idx)
        }
    }

    impl std::ops::Index<std::ops::RangeToInclusive<usize>> for $typ {
        type Output = [u8];

        #[inline]
        fn index(&self, idx: std::ops::RangeToInclusive<usize>) -> &Self::Output {
            self.0.index(idx)
        }
    }

    impl std::iter::IntoIterator for $typ {
        type Item = u8;
        type IntoIter = std::array::IntoIter<u8, $n>;

        fn into_iter(self) -> Self::IntoIter {
            Self::IntoIter::new(self.0)
        }
    }

    impl<'a> std::iter::IntoIterator for &'a $typ {
        type Item = &'a u8;
        type IntoIter = std::slice::Iter<'a, u8>;

        fn into_iter(self) -> Self::IntoIter {
            self.0.iter()
        }
    }

    impl std::fmt::Display for $typ {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write![f, "{}", self.as_str()]
        }
    }
}}

macro_rules! impl_newtype_err {(
    $(#[$meta:meta])*
    $name:ident => $err_msg:expr
) => {
    $(#[$meta])*
    #[derive(Debug)]
    pub struct $name(());

    impl std::error::Error for $name {}

    impl std::fmt::Display for $name {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write![f, $err_msg]
        }
    }
}}
