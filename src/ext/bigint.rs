//! `num-bigint` integers, on the integer line with every primitive.

use alloc::vec::Vec;
use core::fmt;

use num_bigint::{BigInt, BigUint, Sign};

use crate::{
    Decode, Elen, Encode, Error, Integer,
    int::{self, Sign as TokenSign},
    traits::sealed::Sealed,
};

fn encode_magnitude<W: fmt::Write + ?Sized>(
    w: &mut W,
    negative: bool,
    magnitude: &BigUint,
) -> fmt::Result {
    if magnitude.bits() == 0 {
        return w.write_char(crate::ZERO);
    }
    int::write_magnitude(w, negative, magnitude.to_str_radix(10).as_bytes())
}

fn encoded_len(magnitude: &BigUint) -> usize {
    if magnitude.bits() == 0 {
        1
    } else {
        int::magnitude_encoded_len(magnitude.to_str_radix(10).len())
    }
}

/// Reads an integer of any size. Decoding is linear in the input's length for the checks, and
/// then as costly as `BigUint`'s decimal parsing, so bound the length of untrusted input.
fn decode_prefix(s: &str) -> Result<((bool, BigUint), &str), Error> {
    let token = int::read(s, usize::MAX, int::depth(usize::MAX))?;
    let negative = token.sign == TokenSign::Negative;
    let magnitude = match token.sign {
        TokenSign::Zero => BigUint::default(),
        TokenSign::Positive => BigUint::parse_bytes(token.group(), 10).ok_or(Error::OutOfRange)?,
        TokenSign::Negative => {
            let digits: Vec<u8> = token.group().iter().map(|&b| b'9' - b + b'0').collect();
            BigUint::parse_bytes(&digits, 10).ok_or(Error::OutOfRange)?
        }
    };
    Ok(((negative, magnitude), &s[token.end..]))
}

impl Encode for BigUint {
    fn encode_into<W: fmt::Write + ?Sized>(&self, w: &mut W) -> fmt::Result {
        encode_magnitude(w, false, self)
    }

    fn encoded_len(&self) -> usize {
        encoded_len(self)
    }
}

impl Decode for BigUint {
    /// Reads a non-negative integer of any size, failing with [`Error::OutOfRange`] for a
    /// negative one. Bound the length of untrusted input: parsing is superlinear.
    fn decode_prefix(s: &str) -> Result<(Self, &str), Error> {
        match decode_prefix(s)? {
            ((true, _), _) => Err(Error::OutOfRange),
            ((false, magnitude), rest) => Ok((magnitude, rest)),
        }
    }
}

impl Encode for BigInt {
    fn encode_into<W: fmt::Write + ?Sized>(&self, w: &mut W) -> fmt::Result {
        encode_magnitude(w, self.sign() == Sign::Minus, self.magnitude())
    }

    fn encoded_len(&self) -> usize {
        encoded_len(self.magnitude())
    }
}

impl Decode for BigInt {
    /// Reads an integer of any size. Bound the length of untrusted input: parsing is
    /// superlinear.
    fn decode_prefix(s: &str) -> Result<(Self, &str), Error> {
        let ((negative, magnitude), rest) = decode_prefix(s)?;
        let sign = if negative { Sign::Minus } else { Sign::Plus };
        Ok((BigInt::from_biguint(sign, magnitude), rest))
    }
}

impl Sealed for BigUint {}
impl Integer for BigUint {}
impl Sealed for BigInt {}
impl Integer for BigInt {}

macro_rules! conversions {
    ($($t:ty),*) => {$(
        impl From<&$t> for Elen {
            fn from(value: &$t) -> Self {
                Elen::new(value)
            }
        }

        impl From<$t> for Elen {
            fn from(value: $t) -> Self {
                Elen::new(&value)
            }
        }

        impl TryFrom<&Elen> for $t {
            type Error = Error;

            fn try_from(value: &Elen) -> Result<Self, Self::Error> {
                value.decode()
            }
        }
    )*};
}

conversions!(BigInt, BigUint);
