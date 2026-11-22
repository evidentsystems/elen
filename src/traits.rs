use core::fmt;

use crate::Error;

/// A value with an ELEN encoding.
///
/// Encodings of values of one number line compare, byte-wise, in the values' order. Every
/// encoding is self-delimiting, so concatenating the encodings of a tuple's fields sorts in the
/// tuple's lexicographic order; the tuple impls do exactly that.
pub trait Encode {
    /// Writes the encoding.
    fn encode_into<W: fmt::Write + ?Sized>(&self, w: &mut W) -> fmt::Result;

    /// The length of the encoding in bytes.
    fn encoded_len(&self) -> usize;
}

/// A value that can be read back from its ELEN encoding.
///
/// Decoding is strict: only the encoder's own spelling of a value decodes. Whenever
/// `T::decode(s)` returns `Ok(v)`, `v` encodes back to exactly `s`.
pub trait Decode: Sized {
    /// Reads one encoding from the front of `s`, returning the value and the rest of `s`.
    fn decode_prefix(s: &str) -> Result<(Self, &str), Error>;

    /// Reads a value from `s`, which must hold exactly one encoding.
    fn decode(s: &str) -> Result<Self, Error> {
        crate::int::finish(s, Self::decode_prefix(s)?)
    }
}

/// An integer type: its encodings lie on the integer number line, the one [`Elen`] holds.
///
/// This trait is sealed.
///
/// [`Elen`]: crate::Elen
pub trait Integer: Encode + Decode + sealed::Sealed {}

pub(crate) mod sealed {
    pub trait Sealed {}
}

impl<T: Encode + ?Sized> Encode for &T {
    fn encode_into<W: fmt::Write + ?Sized>(&self, w: &mut W) -> fmt::Result {
        (**self).encode_into(w)
    }

    fn encoded_len(&self) -> usize {
        (**self).encoded_len()
    }
}

/// Formats a value as its encoding, without allocating. Made by [`display`](crate::display).
#[derive(Clone, Copy, Debug)]
pub struct Display<'a, T: ?Sized>(pub(crate) &'a T);

impl<T: Encode + ?Sized> fmt::Display for Display<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.encode_into(f)
    }
}

macro_rules! tuple {
    ($($name:ident)+) => {
        impl<$($name: Encode),+> Encode for ($($name,)+) {
            fn encode_into<W: fmt::Write + ?Sized>(&self, w: &mut W) -> fmt::Result {
                #[allow(non_snake_case)]
                let ($($name,)+) = self;
                $($name.encode_into(w)?;)+
                Ok(())
            }

            fn encoded_len(&self) -> usize {
                #[allow(non_snake_case)]
                let ($($name,)+) = self;
                0 $(+ $name.encoded_len())+
            }
        }

        impl<$($name: Decode),+> Decode for ($($name,)+) {
            fn decode_prefix(s: &str) -> Result<(Self, &str), Error> {
                let rest = s;
                $(
                    #[allow(non_snake_case)]
                    let ($name, rest) = $name::decode_prefix(rest).map_err(|e| e.offset(s.len() - rest.len()))?;
                )+
                Ok((($($name,)+), rest))
            }
        }
    };
}

tuple!(A);
tuple!(A B);
tuple!(A B C);
tuple!(A B C D);
tuple!(A B C D E);
tuple!(A B C D E F);
tuple!(A B C D E F G);
tuple!(A B C D E F G H);
