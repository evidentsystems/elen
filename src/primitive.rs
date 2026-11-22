use core::fmt;

use crate::{Decode, Encode, Error, Integer, int, traits::sealed::Sealed};

macro_rules! primitive {
    ($($t:ty),*) => {$(
        impl Encode for $t {
            fn encode_into<W: fmt::Write + ?Sized>(&self, w: &mut W) -> fmt::Result {
                int::encode_primitive(*self, w)
            }

            fn encoded_len(&self) -> usize {
                int::primitive_encoded_len(*self)
            }
        }

        impl Decode for $t {
            /// Reads an integer, failing with [`Error::OutOfRange`] when it does not fit.
            fn decode_prefix(s: &str) -> Result<(Self, &str), Error> {
                int::decode_primitive_prefix(s)
            }
        }

        impl Sealed for $t {}
        impl Integer for $t {}
    )*};
}

primitive!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);
