//! The integer number line (paper §2–§3).
//!
//! A positive number is written as `k` positive markers, then a chain of lengths, innermost
//! first, then its digits: `len(n)` is prefixed whenever it is more than one digit, and so on
//! recursively. `k` counts the groups after the markers. A negative number is the encoding of its
//! magnitude with negative markers and every digit `d` (lengths included) replaced by `9 - d`.
//! Zero is `0`.
//!
//! ```text
//! -10 → --789    -1 → -8    0 → 0    1 → ;1    10 → ;;210    1234567890 → ;;;2101234567890
//! ```

use core::fmt::{self, Write};

use crate::{Error, NEGATIVE, POSITIVE, ZERO};

const POSITIVE_BYTE: u8 = POSITIVE as u8;
const NEGATIVE_BYTE: u8 = NEGATIVE as u8;
const ZERO_BYTE: u8 = ZERO as u8;

/// Number of decimal digits in `n` (1 for 0).
pub(crate) const fn decimal_len(mut n: usize) -> usize {
    let mut len = 1;
    while n >= 10 {
        n /= 10;
        len += 1;
    }
    len
}

/// Number of decimal digits in `n` (1 for 0).
pub(crate) const fn u128_decimal_len(n: u128) -> usize {
    if n <= u64::MAX as u128 {
        let mut n = n as u64;
        let mut len = 1;
        while n >= 10 {
            n /= 10;
            len += 1;
        }
        len
    } else {
        19 + u128_decimal_len(n / TEN_POW_19 as u128)
    }
}

/// Number of markers in the encoding of a magnitude with `len` digits.
pub(crate) const fn depth(len: usize) -> usize {
    let mut depth = 1;
    let mut len = len;
    while len > 1 {
        depth += 1;
        len = decimal_len(len);
    }
    depth
}

/// 10¹⁹, the largest power of ten in a `u64`.
const TEN_POW_19: u64 = 10_000_000_000_000_000_000;

const DIGIT_PAIRS: &[u8; 200] = b"\
    0001020304050607080910111213141516171819\
    2021222324252627282930313233343536373839\
    4041424344454647484950515253545556575859\
    6061626364656667686970717273747576777879\
    8081828384858687888990919293949596979899";

/// Writes the ASCII decimal digits of `n` into `buf` ending at `end`, zero-padded to at least
/// `width` digits, two at a time. Returns where they start.
fn u64_digits(mut n: u64, buf: &mut [u8], mut end: usize, width: usize) -> usize {
    let floor = end - width;
    while n >= 100 {
        let pair = (n % 100) as usize * 2;
        n /= 100;
        end -= 2;
        buf[end..end + 2].copy_from_slice(&DIGIT_PAIRS[pair..pair + 2]);
    }
    if n >= 10 {
        let pair = n as usize * 2;
        end -= 2;
        buf[end..end + 2].copy_from_slice(&DIGIT_PAIRS[pair..pair + 2]);
    } else {
        end -= 1;
        buf[end] = b'0' + n as u8;
    }
    while end > floor {
        end -= 1;
        buf[end] = b'0';
    }
    end
}

/// Writes the ASCII decimal digits of `n` into the end of `buf`, returning them.
pub(crate) fn u128_digits(n: u128, buf: &mut [u8; 39]) -> &[u8] {
    // `u128` division is slow; split off 19-digit chunks so the per-digit work is on `u64`s.
    let mut end = buf.len();
    let mut high = n;
    let low = loop {
        if let Ok(low) = u64::try_from(high) {
            break low;
        }
        let chunk = (high % TEN_POW_19 as u128) as u64;
        high /= TEN_POW_19 as u128;
        end = u64_digits(chunk, buf, end, 19);
    };
    let start = u64_digits(low, buf, end, 1);
    &buf[start..]
}

/// Buffers ASCII bytes on the stack, so a writer sees a few `write_str` calls rather than one per
/// byte.
pub(crate) struct Out<'w, W: Write + ?Sized> {
    w: &'w mut W,
    buf: [u8; 64],
    len: usize,
}

impl<'w, W: Write + ?Sized> Out<'w, W> {
    pub(crate) fn new(w: &'w mut W) -> Self {
        Out {
            w,
            buf: [0; 64],
            len: 0,
        }
    }

    pub(crate) fn push(&mut self, b: u8) -> fmt::Result {
        if self.len == self.buf.len() {
            self.flush()?;
        }
        self.buf[self.len] = b;
        self.len += 1;
        Ok(())
    }

    /// Pushes ASCII `digits`, replacing each `d` with `9 - d` when `invert`.
    pub(crate) fn digits(&mut self, mut digits: &[u8], invert: bool) -> fmt::Result {
        while !digits.is_empty() {
            if self.len == self.buf.len() {
                self.flush()?;
            }
            let n = digits.len().min(self.buf.len() - self.len);
            let (chunk, rest) = digits.split_at(n);
            let out = &mut self.buf[self.len..self.len + n];
            if invert {
                for (o, &d) in out.iter_mut().zip(chunk) {
                    *o = b'9' - d + b'0';
                }
            } else {
                out.copy_from_slice(chunk);
            }
            self.len += n;
            digits = rest;
        }
        Ok(())
    }

    /// Pushes `n` zeros, or nines when `invert`.
    pub(crate) fn zeros(&mut self, n: usize, invert: bool) -> fmt::Result {
        let digit = if invert { b'9' } else { b'0' };
        for _ in 0..n {
            self.push(digit)?;
        }
        Ok(())
    }

    pub(crate) fn flush(&mut self) -> fmt::Result {
        let s = core::str::from_utf8(&self.buf[..self.len]).map_err(|_| fmt::Error)?;
        self.w.write_str(s)?;
        self.len = 0;
        Ok(())
    }
}

/// Writes the encoding of a nonzero magnitude given as ASCII `digits` without a leading zero.
pub(crate) fn write_magnitude<W: Write + ?Sized>(
    w: &mut W,
    negative: bool,
    digits: &[u8],
) -> fmt::Result {
    let mut out = Out::new(w);
    push_magnitude(&mut out, negative, digits, 0)?;
    out.flush()
}

/// Pushes the encoding of a nonzero magnitude: ASCII `digits` without a leading zero, followed by
/// `zeros` more zeros.
pub(crate) fn push_magnitude<W: Write + ?Sized>(
    out: &mut Out<'_, W>,
    negative: bool,
    digits: &[u8],
    zeros: usize,
) -> fmt::Result {
    debug_assert!(!digits.is_empty() && digits[0] != b'0');
    // The length chain: `chain[0]` is the number of digits, each later entry the length of the
    // one before, stopping at the first single-digit length. A `usize` has at most 20 digits, so
    // the chain is at most 3 long.
    let mut chain = [0usize; 4];
    let mut links = 0;
    let mut len = digits.len() + zeros;
    while len > 1 {
        chain[links] = len;
        links += 1;
        len = decimal_len(len);
    }
    let marker = if negative {
        NEGATIVE_BYTE
    } else {
        POSITIVE_BYTE
    };
    for _ in 0..=links {
        out.push(marker)?;
    }
    let mut buf = [0u8; 39];
    for &link in chain[..links].iter().rev() {
        out.digits(u128_digits(link as u128, &mut buf), negative)?;
    }
    out.digits(digits, negative)?;
    out.zeros(zeros, negative)
}

/// Length of the encoding of a nonzero magnitude with `len` digits.
pub(crate) fn magnitude_encoded_len(len: usize) -> usize {
    let mut total = 1 + len;
    let mut len = len;
    while len > 1 {
        len = decimal_len(len);
        total += 1 + len;
    }
    total
}

/// The sign of a decoded integer token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sign {
    Negative,
    Zero,
    Positive,
}

/// A canonical integer token read from the front of a string.
pub(crate) struct Token<'a> {
    pub(crate) sign: Sign,
    /// The magnitude's digit group as written (inverted when negative); empty for zero.
    group: &'a [u8],
    /// Byte offset just past the token.
    pub(crate) end: usize,
}

impl<'a> Token<'a> {
    /// The magnitude's digit group as written: inverted when negative, empty for zero.
    pub(crate) fn group(&self) -> &'a [u8] {
        self.group
    }

    /// The magnitude as a `u128`, if it fits.
    pub(crate) fn magnitude_u128(&self) -> Option<u128> {
        let negative = self.sign == Sign::Negative;
        let value = |b: u8| (if negative { b'9' - b } else { b - b'0' }) as u64;
        // Up to 19 digits always fit a `u64`; only longer magnitudes need `u128` arithmetic.
        let (head, tail) = self.group.split_at(self.group.len().saturating_sub(19));
        let tail_value = tail.iter().fold(0u64, |acc, &b| acc * 10 + value(b));
        if head.is_empty() {
            return Some(tail_value as u128);
        }
        head.iter()
            .try_fold(0u128, |acc, &b| {
                acc.checked_mul(10)?.checked_add(value(b) as u128)
            })?
            .checked_mul(10u128.pow(tail.len() as u32))?
            .checked_add(tail_value as u128)
    }
}

pub(crate) fn unexpected(s: &str, at: usize) -> Error {
    match s[at..].chars().next() {
        Some(found) => Error::UnexpectedChar { at, found },
        None => Error::UnexpectedEnd,
    }
}

/// Reads one canonical integer token from the front of `s`, refusing (with
/// [`Error::OutOfRange`]) any magnitude longer than `max_digits` before reading it. `max_depth`
/// is `depth(max_digits)`.
pub(crate) fn read(s: &str, max_digits: usize, max_depth: usize) -> Result<Token<'_>, Error> {
    let bytes = s.as_bytes();
    let marker = match bytes.first() {
        None => return Err(Error::Empty),
        Some(&ZERO_BYTE) => {
            return Ok(Token {
                sign: Sign::Zero,
                group: &[],
                end: 1,
            });
        }
        Some(&b) if b == POSITIVE_BYTE || b == NEGATIVE_BYTE => b,
        Some(_) => return Err(unexpected(s, 0)),
    };
    let negative = marker == NEGATIVE_BYTE;
    let groups = bytes.iter().take_while(|&&b| b == marker).count();
    if groups > max_depth {
        return Err(Error::OutOfRange);
    }
    let digit = |b: u8| if negative { b'9' - b } else { b - b'0' };

    let mut pos = groups;
    let mut width = 1usize;
    let mut level = 1;
    loop {
        let end = pos.checked_add(width).ok_or(Error::OutOfRange)?;
        let group = &bytes[pos..bytes.len().min(end)];
        if let Some(i) = group.iter().position(|b| !b.is_ascii_digit()) {
            return Err(unexpected(s, pos + i));
        }
        if group.len() < width {
            return Err(Error::UnexpectedEnd);
        }
        // A leading zero, a lone zero magnitude (zero is `0`), and a zero length are all
        // spellings the encoder never writes.
        if digit(group[0]) == 0 {
            return Err(Error::NonCanonical { at: pos });
        }
        if level == groups {
            return Ok(Token {
                sign: if negative {
                    Sign::Negative
                } else {
                    Sign::Positive
                },
                group,
                end,
            });
        }
        let next = group.iter().try_fold(0usize, |acc, &b| {
            acc.checked_mul(10)?.checked_add(digit(b) as usize)
        });
        let next = next.ok_or(Error::OutOfRange)?;
        // The encoder only prefixes a length of two or more digits.
        if next < 2 {
            return Err(Error::NonCanonical { at: pos });
        }
        if next > max_digits {
            return Err(Error::OutOfRange);
        }
        width = next;
        pos = end;
        level += 1;
    }
}

/// A primitive integer type, seen as a sign and a `u128` magnitude.
pub(crate) trait Primitive: Sized + Copy {
    /// Digits in the largest magnitude of the type.
    const MAX_DIGITS: usize;
    /// Markers in the deepest encoding of the type.
    const MAX_DEPTH: usize = depth(Self::MAX_DIGITS);
    fn into_parts(self) -> (bool, u128);
    fn from_parts(negative: bool, magnitude: u128) -> Option<Self>;
}

macro_rules! unsigned {
    ($($t:ty),*) => {$(
        impl Primitive for $t {
            const MAX_DIGITS: usize = u128_decimal_len(<$t>::MAX as u128);
            fn into_parts(self) -> (bool, u128) {
                (false, self as u128)
            }
            fn from_parts(negative: bool, magnitude: u128) -> Option<Self> {
                if negative { None } else { <$t>::try_from(magnitude).ok() }
            }
        }
    )*};
}

macro_rules! signed {
    ($($t:ty),*) => {$(
        impl Primitive for $t {
            const MAX_DIGITS: usize = u128_decimal_len(<$t>::MIN.unsigned_abs() as u128);
            fn into_parts(self) -> (bool, u128) {
                (self < 0, self.unsigned_abs() as u128)
            }
            fn from_parts(negative: bool, magnitude: u128) -> Option<Self> {
                if !negative {
                    <$t>::try_from(magnitude).ok()
                } else if magnitude <= <$t>::MIN.unsigned_abs() as u128 {
                    // `magnitude as $t` wraps `|MIN|` to `MIN`, whose negation is itself.
                    Some((0 as $t).wrapping_sub(magnitude as $t))
                } else {
                    None
                }
            }
        }
    )*};
}

unsigned!(u8, u16, u32, u64, u128, usize);
signed!(i8, i16, i32, i64, i128, isize);

pub(crate) fn encode_primitive<T: Primitive, W: Write + ?Sized>(v: T, w: &mut W) -> fmt::Result {
    let (negative, magnitude) = v.into_parts();
    if magnitude == 0 {
        return w.write_char(ZERO);
    }
    let mut buf = [0u8; 39];
    write_magnitude(w, negative, u128_digits(magnitude, &mut buf))
}

/// Pushes the encoding of `v`.
pub(crate) fn push_i128<W: Write + ?Sized>(out: &mut Out<'_, W>, v: i128) -> fmt::Result {
    if v == 0 {
        return out.push(ZERO_BYTE);
    }
    let mut buf = [0u8; 39];
    push_magnitude(out, v < 0, u128_digits(v.unsigned_abs(), &mut buf), 0)
}

pub(crate) fn primitive_encoded_len<T: Primitive>(v: T) -> usize {
    let (_, magnitude) = v.into_parts();
    if magnitude == 0 {
        1
    } else {
        magnitude_encoded_len(u128_decimal_len(magnitude))
    }
}

pub(crate) fn decode_primitive_prefix<T: Primitive>(s: &str) -> Result<(T, &str), Error> {
    let token = read(s, T::MAX_DIGITS, T::MAX_DEPTH)?;
    let value = match token.sign {
        Sign::Zero => T::from_parts(false, 0),
        sign => token
            .magnitude_u128()
            .and_then(|m| T::from_parts(sign == Sign::Negative, m)),
    };
    Ok((value.ok_or(Error::OutOfRange)?, &s[token.end..]))
}

/// Fails with [`Error::TrailingInput`] unless `rest`, the unread tail of `s`, is empty.
pub(crate) fn finish<T>(s: &str, (value, rest): (T, &str)) -> Result<T, Error> {
    if rest.is_empty() {
        Ok(value)
    } else {
        Err(Error::TrailingInput {
            at: s.len() - rest.len(),
        })
    }
}
