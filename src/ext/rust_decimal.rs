//! `rust_decimal::Decimal`, on the real line, and in fixed-point form as [`Fixed<Decimal>`].
//!
//! [`Fixed<Decimal>`]: crate::Fixed

use rust_decimal::Decimal;

use crate::{
    Error, int,
    real::{Read, Real, Value, scientific},
};

/// The largest scale a `Decimal` holds.
const MAX_SCALE: i128 = 28;

impl Real for Decimal {
    // A 96-bit mantissa has at most 29 digits.
    const MAX_DIGITS: usize = 29;

    fn with_value<R>(&self, f: impl FnOnce(Value<'_>) -> R) -> R {
        if self.is_zero() {
            return f(Value::Zero);
        }
        let normal = self.normalize();
        let mut buf = [0u8; 39];
        let digits = int::u128_digits(normal.mantissa().unsigned_abs(), &mut buf);
        // `normalize` leaves an integer's trailing zeros in its mantissa.
        let exponent = digits.len() as i128 - normal.scale() as i128;
        f(Value::Finite {
            negative: normal.is_sign_negative(),
            digits: crate::real::trim_zeros(digits),
            exponent,
        })
    }

    fn from_read(read: Read<'_>) -> Result<Self, Error> {
        let digits = match read {
            Read::Zero => return Ok(Decimal::ZERO),
            Read::NegInfinity | Read::PosInfinity => return Err(Error::OutOfRange),
            Read::Finite(digits) => digits,
        };
        let mut mantissa = digits.magnitude_u128().ok_or(Error::OutOfRange)?;
        let scale = if digits.exp10 > 0 {
            if digits.exp10 > MAX_SCALE {
                return Err(Error::OutOfRange);
            }
            mantissa = mantissa
                .checked_mul(10u128.pow(digits.exp10 as u32))
                .ok_or(Error::OutOfRange)?;
            0
        } else if -digits.exp10 <= MAX_SCALE {
            (-digits.exp10) as u32
        } else {
            return Err(Error::OutOfRange);
        };
        let mantissa = i128::try_from(mantissa).map_err(|_| Error::OutOfRange)?;
        let mantissa = if digits.negative { -mantissa } else { mantissa };
        Decimal::try_from_i128_with_scale(mantissa, scale).map_err(|_| Error::OutOfRange)
    }
}

scientific!(Decimal);
