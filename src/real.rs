//! The real number line (paper §6), fixed-point decimals (§4–§5), and floats.
//!
//! A nonzero real `±0.d₁d₂…dₙ × 10ᵉ`, with `d₁` and `dₙ` nonzero, is written as a sign marker,
//! then the exponent `e` as an integer (negated for a negative number), then the digits
//! (inverted for a negative number), then a terminator that sorts below every digit for a
//! positive number and above every digit for a negative one. Zero is `0`, and the infinities
//! are [`NEG_INFINITY`] and [`POS_INFINITY`], which sort below and above every other real.
//!
//! ```text
//! -0.1×10¹¹ → ---7888;       0.00123 → ;-7123-
//!      -1.4 → --885;           0.123 → ;0123-
//!        -1 → --88;                1 → ;;11-
//!    -0.123 → -0876;             1.4 → ;;114-
//!         0 → 0            0.1×10¹¹ → ;;;2111-
//! ```
//!
//! Every type on this line (`f32`, `f64`, and with features `rust_decimal::Decimal` and
//! `bigdecimal::BigDecimal`) writes a value the same way, so encodings of different real types
//! compare correctly. Reals and integers are different lines: `1.0` is `;;11-`, while `1` is `;1`.

use core::fmt::{self, Write};

use crate::{
    Decode, Encode, Error, NEGATIVE, POSITIVE, ZERO,
    int::{self, Out},
};

/// The encoding of negative infinity, below every other real.
pub const NEG_INFINITY: char = '!';
/// The encoding of positive infinity, above every other real.
pub const POS_INFINITY: char = '~';

const POSITIVE_BYTE: u8 = POSITIVE as u8;
const NEGATIVE_BYTE: u8 = NEGATIVE as u8;

/// A real, as a type hands it to the encoder.
pub(crate) enum Value<'a> {
    NegInfinity,
    Zero,
    PosInfinity,
    /// `±0.digits × 10^exponent`; `digits` are ASCII with no leading or trailing zero.
    Finite {
        negative: bool,
        digits: &'a [u8],
        exponent: i128,
    },
}

/// A real, as the decoder hands it to a type.
pub(crate) enum Read<'a> {
    NegInfinity,
    Zero,
    PosInfinity,
    Finite(Digits<'a>),
}

/// A nonzero finite real `±int(digits) × 10^exp10`, its digits in up to two groups as written
/// (inverted when negative). The digits may have leading zeros but are not all zero.
pub(crate) struct Digits<'a> {
    pub(crate) negative: bool,
    groups: [&'a [u8]; 2],
    pub(crate) exp10: i128,
}

impl Digits<'_> {
    /// The ASCII digits, most significant first.
    pub(crate) fn ascii(&self) -> impl Iterator<Item = u8> + '_ {
        let negative = self.negative;
        self.groups
            .into_iter()
            .flatten()
            .map(move |&b| if negative { b'9' - b + b'0' } else { b })
    }

    pub(crate) fn len(&self) -> usize {
        self.groups[0].len() + self.groups[1].len()
    }

    /// The digits as a `u128`, if they fit.
    #[cfg(feature = "rust_decimal")]
    pub(crate) fn magnitude_u128(&self) -> Option<u128> {
        self.ascii().try_fold(0u128, |acc, d| {
            acc.checked_mul(10)?.checked_add((d - b'0') as u128)
        })
    }
}

/// `digits` without their trailing zeros.
#[cfg(any(feature = "rust_decimal", feature = "bigdecimal"))]
pub(crate) fn trim_zeros(digits: &[u8]) -> &[u8] {
    let len = digits.iter().rposition(|&d| d != b'0').map_or(0, |i| i + 1);
    &digits[..len]
}

/// A type on the real line.
pub(crate) trait Real: Sized {
    /// The most digits any group of an encoding of this type can have.
    const MAX_DIGITS: usize;
    fn with_value<R>(&self, f: impl FnOnce(Value<'_>) -> R) -> R;
    /// The value read, or [`Error::OutOfRange`] when the type cannot hold it exactly.
    fn from_read(read: Read<'_>) -> Result<Self, Error>;
}

fn digit(negative: bool, b: u8) -> u8 {
    if negative { b'9' - b } else { b - b'0' }
}

fn sign_marker(negative: bool) -> u8 {
    if negative {
        NEGATIVE_BYTE
    } else {
        POSITIVE_BYTE
    }
}

/// The terminator after a fraction: below every digit when positive, above when negative.
fn terminator(negative: bool) -> u8 {
    sign_marker(!negative)
}

fn write_special<W: Write + ?Sized>(w: &mut W, value: &Value<'_>) -> Option<fmt::Result> {
    match value {
        Value::NegInfinity => Some(w.write_char(NEG_INFINITY)),
        Value::Zero => Some(w.write_char(ZERO)),
        Value::PosInfinity => Some(w.write_char(POS_INFINITY)),
        Value::Finite { .. } => None,
    }
}

fn exponent_len(e: i128) -> usize {
    if e == 0 {
        1
    } else {
        int::magnitude_encoded_len(int::u128_decimal_len(e.unsigned_abs()))
    }
}

pub(crate) fn write_scientific<W: Write + ?Sized>(w: &mut W, value: Value<'_>) -> fmt::Result {
    if let Some(done) = write_special(w, &value) {
        return done;
    }
    let Value::Finite {
        negative,
        digits,
        exponent,
    } = value
    else {
        unreachable!()
    };
    let mut out = Out::new(w);
    out.push(sign_marker(negative))?;
    int::push_i128(&mut out, if negative { -exponent } else { exponent })?;
    out.digits(digits, negative)?;
    out.push(terminator(negative))?;
    out.flush()
}

pub(crate) fn scientific_len(value: Value<'_>) -> usize {
    match value {
        Value::Finite {
            digits, exponent, ..
        } => 1 + exponent_len(exponent) + digits.len() + 1,
        _ => 1,
    }
}

/// Reads the digits of a fraction starting at `pos`, and its terminator. Returns the digits as
/// written and the offset past the terminator.
fn read_fraction(
    s: &str,
    pos: usize,
    negative: bool,
    max_digits: usize,
) -> Result<(&[u8], usize), Error> {
    let bytes = s.as_bytes();
    let len = bytes[pos..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if len > max_digits {
        return Err(Error::OutOfRange);
    }
    let end = pos + len;
    match bytes.get(end) {
        None => return Err(Error::UnexpectedEnd),
        Some(&b) if b == terminator(negative) => {}
        Some(_) => return Err(int::unexpected(s, end)),
    }
    let group = &bytes[pos..end];
    if let Some(&last) = group.last()
        && digit(negative, last) == 0
    {
        return Err(Error::NonCanonical { at: end - 1 });
    }
    Ok((group, end + 1))
}

/// Reads a special value, or the sign of a finite one.
fn read_sign(s: &str) -> Result<Result<Read<'static>, bool>, Error> {
    match s.as_bytes().first() {
        None => Err(Error::Empty),
        Some(b'0') => Ok(Ok(Read::Zero)),
        Some(b'!') => Ok(Ok(Read::NegInfinity)),
        Some(b'~') => Ok(Ok(Read::PosInfinity)),
        Some(&POSITIVE_BYTE) => Ok(Err(false)),
        Some(&NEGATIVE_BYTE) => Ok(Err(true)),
        Some(_) => Err(int::unexpected(s, 0)),
    }
}

pub(crate) fn read_scientific(s: &str, max_digits: usize) -> Result<(Read<'_>, usize), Error> {
    let negative = match read_sign(s)? {
        Ok(special) => return Ok((special, 1)),
        Err(negative) => negative,
    };
    let (exponent, rest) = i128::decode_prefix(&s[1..]).map_err(|e| e.offset(1))?;
    let exponent = if negative {
        exponent.checked_neg().ok_or(Error::OutOfRange)?
    } else {
        exponent
    };
    let pos = s.len() - rest.len();
    let (mantissa, end) = read_fraction(s, pos, negative, max_digits)?;
    match mantissa.first() {
        Some(&first) if digit(negative, first) != 0 => {}
        _ => return Err(Error::NonCanonical { at: pos }),
    }
    let exp10 = exponent
        .checked_sub(mantissa.len() as i128)
        .ok_or(Error::OutOfRange)?;
    let digits = Digits {
        negative,
        groups: [mantissa, &[]],
        exp10,
    };
    Ok((Read::Finite(digits), end))
}

/// Writes a real in fixed-point form (paper §5): the signed integer part (§3), then the
/// fraction's digits (§4, inverted when negative), then a terminator. A number below one in
/// magnitude writes its zero integer part as a sign and a zero digit.
pub(crate) fn write_fixed<W: Write + ?Sized>(w: &mut W, value: Value<'_>) -> fmt::Result {
    if let Some(done) = write_special(w, &value) {
        return done;
    }
    let Value::Finite {
        negative,
        digits,
        exponent,
    } = value
    else {
        unreachable!()
    };
    let mut out = Out::new(w);
    if exponent > 0 {
        let exponent = usize::try_from(exponent).map_err(|_| fmt::Error)?;
        let integer = exponent.min(digits.len());
        int::push_magnitude(&mut out, negative, &digits[..integer], exponent - integer)?;
        out.digits(&digits[integer..], negative)?;
    } else {
        let zeros = usize::try_from(-exponent).map_err(|_| fmt::Error)?;
        out.push(sign_marker(negative))?;
        out.zeros(1 + zeros, negative)?;
        out.digits(digits, negative)?;
    }
    out.push(terminator(negative))?;
    out.flush()
}

pub(crate) fn fixed_len(value: Value<'_>) -> usize {
    match value {
        Value::Finite {
            digits, exponent, ..
        } if exponent > 0 => {
            let exponent = exponent as usize;
            let integer = exponent.min(digits.len());
            int::magnitude_encoded_len(exponent) + (digits.len() - integer) + 1
        }
        Value::Finite {
            digits, exponent, ..
        } => 2 + (-exponent) as usize + digits.len() + 1,
        _ => 1,
    }
}

pub(crate) fn read_fixed(s: &str, max_digits: usize) -> Result<(Read<'_>, usize), Error> {
    let negative = match read_sign(s)? {
        Ok(special) => return Ok((special, 1)),
        Err(negative) => negative,
    };
    let bytes = s.as_bytes();
    let (integer, fraction, end) = match bytes.get(1) {
        // A zero integer part; the canonical integer encoding never has a zero digit here.
        Some(&b) if b.is_ascii_digit() && digit(negative, b) == 0 => {
            let (fraction, end) = read_fraction(s, 2, negative, max_digits)?;
            if fraction.is_empty() {
                return Err(Error::NonCanonical { at: 1 });
            }
            (&[][..], fraction, end)
        }
        _ => {
            let token = int::read(s, max_digits, int::depth(max_digits))?;
            let (fraction, end) = read_fraction(s, token.end, negative, max_digits)?;
            (token.group(), fraction, end)
        }
    };
    let digits = Digits {
        negative,
        groups: [integer, fraction],
        exp10: -(fraction.len() as i128),
    };
    Ok((Read::Finite(digits), end))
}

/// A real in fixed-point form (paper §5): its integer part, then its fraction.
///
/// This is a separate line from the default, scientific, real encoding. It is longer for very
/// large or very small magnitudes but reads as an ordinary decimal: `-10.5` is `--7894;` and
/// `100.5` is `;;31005-`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed<T>(pub T);

impl<T: Real> Encode for Fixed<T> {
    fn encode_into<W: Write + ?Sized>(&self, w: &mut W) -> fmt::Result {
        self.0.with_value(|v| write_fixed(w, v))
    }

    fn encoded_len(&self) -> usize {
        self.0.with_value(fixed_len)
    }
}

impl<T: Real> Decode for Fixed<T> {
    fn decode_prefix(s: &str) -> Result<(Self, &str), Error> {
        let (read, end) = read_fixed(s, T::MAX_DIGITS)?;
        Ok((Fixed(T::from_read(read)?), &s[end..]))
    }
}

/// Implements `Encode` and `Decode` on the scientific real line for a `Real` type.
macro_rules! scientific {
    ($($t:ty),*) => {$(
        impl crate::Encode for $t {
            fn encode_into<W: core::fmt::Write + ?Sized>(&self, w: &mut W) -> core::fmt::Result {
                crate::real::Real::with_value(self, |v| crate::real::write_scientific(w, v))
            }

            fn encoded_len(&self) -> usize {
                crate::real::Real::with_value(self, crate::real::scientific_len)
            }
        }

        impl crate::Decode for $t {
            /// Reads a real, failing with [`Error::OutOfRange`](crate::Error::OutOfRange)
            /// when the type cannot hold it exactly.
            fn decode_prefix(s: &str) -> Result<(Self, &str), crate::Error> {
                let (read, end) = crate::real::read_scientific(
                    s,
                    <$t as crate::real::Real>::MAX_DIGITS,
                )?;
                Ok((<$t as crate::real::Real>::from_read(read)?, &s[end..]))
            }
        }
    )*};
}
#[cfg(any(feature = "rust_decimal", feature = "bigdecimal"))]
pub(crate) use scientific;

/// A float known not to be NaN, so it has an encoding.
///
/// `-0.0` encodes as `0`, the same as `0.0`, and decodes as `0.0`.
///
/// ```
/// # #[cfg(feature = "alloc")] {
/// use elen::NotNan;
///
/// let key = elen::encode(&(7u32, NotNan::new(-1.5f64)?));
/// assert_eq!(key, ";7--884;");
/// assert_eq!(elen::decode::<(u32, f64)>(&key)?, (7, -1.5));
/// assert_eq!(NotNan::new(f64::NAN), Err(elen::Error::NaN));
/// # }
/// # Ok::<(), elen::Error>(())
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct NotNan<F>(F);

/// A primitive float type: `f32` or `f64`. This trait is sealed.
pub trait Float: Copy + sealed::Sealed {
    #[doc(hidden)]
    const ZERO: Self;
    #[doc(hidden)]
    const NEG_INFINITY: Self;
    #[doc(hidden)]
    const INFINITY: Self;
    #[doc(hidden)]
    fn parse(s: &str) -> Option<Self>;
    #[doc(hidden)]
    fn negate(self) -> Self;
    #[doc(hidden)]
    fn is_nan(self) -> bool;
}

impl<F: Float> NotNan<F> {
    /// `value`, unless it is NaN.
    pub fn new(value: F) -> Result<Self, Error> {
        if value.is_nan() {
            Err(Error::NaN)
        } else {
            Ok(NotNan(value))
        }
    }

    /// The float.
    pub fn get(self) -> F {
        self.0
    }
}

mod sealed {
    pub trait Sealed {}
}

/// A stack buffer for formatting a float.
struct Buf {
    bytes: [u8; 64],
    len: usize,
}

impl Buf {
    fn new() -> Self {
        Buf {
            bytes: [0; 64],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or_default()
    }
}

impl Write for Buf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        self.bytes
            .get_mut(self.len..end)
            .ok_or(fmt::Error)?
            .copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// Splits `{:e}` output (`d.ddde-x`) of a positive float into its shortest digits and exponent.
fn shortest(formatted: &str, digits: &mut [u8; 17]) -> (usize, i128) {
    let (mantissa, exponent) = formatted.split_once('e').unwrap_or((formatted, "0"));
    let mut len = 0;
    for b in mantissa.bytes().filter(u8::is_ascii_digit) {
        digits[len] = b;
        len += 1;
    }
    while len > 1 && digits[len - 1] == b'0' {
        len -= 1;
    }
    let exponent: i128 = exponent.parse().unwrap_or(0);
    (len, exponent + 1)
}

macro_rules! float {
    ($($t:ty => $max_digits:expr),*) => {$(
        impl sealed::Sealed for $t {}
        impl Float for $t {
            const ZERO: Self = 0.0;
            const NEG_INFINITY: Self = <$t>::NEG_INFINITY;
            const INFINITY: Self = <$t>::INFINITY;

            fn parse(s: &str) -> Option<Self> {
                s.parse().ok()
            }

            fn negate(self) -> Self {
                -self
            }

            fn is_nan(self) -> bool {
                <$t>::is_nan(self)
            }
        }

        impl TryFrom<$t> for NotNan<$t> {
            type Error = Error;

            fn try_from(value: $t) -> Result<Self, Error> {
                NotNan::<$t>::new(value)
            }
        }

        impl From<NotNan<$t>> for $t {
            fn from(value: NotNan<$t>) -> Self {
                value.0
            }
        }

        impl Real for NotNan<$t> {
            const MAX_DIGITS: usize = $max_digits;

            fn with_value<R>(&self, f: impl FnOnce(Value<'_>) -> R) -> R {
                let v = self.0;
                if v == 0.0 {
                    return f(Value::Zero);
                }
                if v.is_infinite() {
                    return f(if v < 0.0 { Value::NegInfinity } else { Value::PosInfinity });
                }
                let mut buf = Buf::new();
                write!(buf, "{:e}", v.abs()).expect("a float's {:e} form fits 64 bytes");
                let mut digits = [0u8; 17];
                let (len, exponent) = shortest(buf.as_str(), &mut digits);
                f(Value::Finite { negative: v < 0.0, digits: &digits[..len], exponent })
            }

            fn from_read(read: Read<'_>) -> Result<Self, Error> {
                let digits = match read {
                    Read::NegInfinity => return Ok(NotNan(<$t>::NEG_INFINITY)),
                    Read::Zero => return Ok(NotNan(0.0)),
                    Read::PosInfinity => return Ok(NotNan(<$t>::INFINITY)),
                    Read::Finite(digits) => digits,
                };
                let mut buf = Buf::new();
                for d in digits.ascii() {
                    buf.write_char(d as char).map_err(|_| Error::OutOfRange)?;
                }
                write!(buf, "e{}", digits.exp10).map_err(|_| Error::OutOfRange)?;
                let magnitude: $t = buf.as_str().parse().map_err(|_| Error::OutOfRange)?;
                let value = NotNan(if digits.negative { -magnitude } else { magnitude });
                // The encoding was canonical, so it is the value's own encoding exactly when the
                // float holds the number exactly.
                let exact = value.with_value(|v| match v {
                    Value::Finite { negative, digits: d, exponent } => {
                        negative == digits.negative
                            && d.len() == digits.len()
                            && exponent - d.len() as i128 == digits.exp10
                            && d.iter().copied().eq(digits.ascii())
                    }
                    _ => false,
                });
                if exact { Ok(value) } else { Err(Error::OutOfRange) }
            }
        }

        impl Decode for $t {
            /// Reads a real, failing with [`Error::OutOfRange`] unless the float holds it
            /// exactly. [`decode_rounded`] rounds instead.
            fn decode_prefix(s: &str) -> Result<(Self, &str), Error> {
                let (read, end) = read_scientific(s, $max_digits)?;
                Ok((NotNan::<$t>::from_read(read)?.get(), &s[end..]))
            }
        }
    )*};
}

// The shortest round-trip forms have at most 9 and 17 significant digits.
float!(f32 => 9, f64 => 17);

scientific!(NotNan<f32>, NotNan<f64>);

/// Reads any real-line encoding as the nearest float, where [`Decode`] would refuse a number the
/// float cannot hold exactly: for example, a [`Decimal`] with more digits than an `f64` keeps.
///
/// Magnitudes beyond the float's range round to an infinity or zero.
///
/// ```
/// let s = ";0333333333333333333333333333-"; // 0.333…, as a 27-digit decimal would write it
/// assert_eq!(elen::decode::<f64>(s), Err(elen::Error::OutOfRange));
/// assert_eq!(elen::decode_rounded::<f64>(s), Ok(1.0 / 3.0));
/// ```
///
/// [`Decimal`]: https://docs.rs/rust_decimal
#[cfg(feature = "alloc")]
pub fn decode_rounded<F: Float>(s: &str) -> Result<F, Error> {
    let (read, end) = read_scientific(s, usize::MAX)?;
    int::finish(s, ((), &s[end..]))?;
    let digits = match read {
        Read::NegInfinity => return Ok(F::NEG_INFINITY),
        Read::Zero => return Ok(F::ZERO),
        Read::PosInfinity => return Ok(F::INFINITY),
        Read::Finite(digits) => digits,
    };
    let mut text = alloc::string::String::with_capacity(digits.len() + 42);
    text.extend(digits.ascii().map(char::from));
    write!(text, "e{}", digits.exp10).map_err(|_| Error::OutOfRange)?;
    let magnitude = F::parse(&text).ok_or(Error::OutOfRange)?;
    Ok(if digits.negative {
        magnitude.negate()
    } else {
        magnitude
    })
}
