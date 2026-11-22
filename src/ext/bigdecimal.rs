//! `bigdecimal::BigDecimal`, on the real line, and in fixed-point form as
//! [`Fixed<BigDecimal>`].
//!
//! [`Fixed<BigDecimal>`]: crate::Fixed

use alloc::vec::Vec;

use bigdecimal::{
    BigDecimal,
    num_bigint::{BigInt, BigUint, Sign},
};

use crate::{
    Error,
    real::{Read, Real, Value, scientific, trim_zeros},
};

impl Real for BigDecimal {
    const MAX_DIGITS: usize = usize::MAX;

    fn with_value<R>(&self, f: impl FnOnce(Value<'_>) -> R) -> R {
        let (int, scale) = self.as_bigint_and_exponent();
        if int.sign() == Sign::NoSign {
            return f(Value::Zero);
        }
        let digits = int.magnitude().to_str_radix(10);
        let exponent = digits.len() as i128 - scale as i128;
        f(Value::Finite {
            negative: int.sign() == Sign::Minus,
            digits: trim_zeros(digits.as_bytes()),
            exponent,
        })
    }

    /// Reads a real of any size. Bound the length of untrusted input: parsing is superlinear.
    fn from_read(read: Read<'_>) -> Result<Self, Error> {
        let digits = match read {
            Read::Zero => return Ok(BigDecimal::default()),
            Read::NegInfinity | Read::PosInfinity => return Err(Error::OutOfRange),
            Read::Finite(digits) => digits,
        };
        let ascii: Vec<u8> = digits.ascii().collect();
        let magnitude = BigUint::parse_bytes(&ascii, 10).ok_or(Error::OutOfRange)?;
        let sign = if digits.negative {
            Sign::Minus
        } else {
            Sign::Plus
        };
        let scale = i64::try_from(-digits.exp10).map_err(|_| Error::OutOfRange)?;
        Ok(BigDecimal::new(
            BigInt::from_biguint(sign, magnitude),
            scale,
        ))
    }
}

scientific!(BigDecimal);
