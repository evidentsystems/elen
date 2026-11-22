use std::{fmt::Display, num::TryFromIntError, str::FromStr};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ElenError {
    #[error("{0} is not a valid ELEN string")]
    InvalidString(String),

    #[error("decode error: {0}")]
    DecodeError(#[from] TryFromIntError),

    #[error("invalid signed integer format: {0}")]
    InvalidSignedFormat(String),
}

pub struct Elen(String);
const LENGTH_RUNE: &str = ";";

/// Prefix for negative signed integers in ELEN encoding.
/// Using '!' (ASCII 33) ensures negative numbers sort before positive ones lexicographically,
/// since '!' < '~' in ASCII.
const NEGATIVE_PREFIX: char = '!';

/// Prefix for non-negative signed integers in ELEN encoding.
/// Using '~' (ASCII 126) ensures non-negative numbers sort after negative ones lexicographically.
const NON_NEGATIVE_PREFIX: char = '~';

/// Encode a signed 128-bit integer to a lexicographically sortable string.
///
/// The encoding uses a prefix to ensure correct sort order:
/// - Negative numbers are prefixed with '-' and use a complemented representation
///   so that larger (less negative) numbers have larger encoded values.
/// - Non-negative numbers are prefixed with '+' and encoded normally.
///
/// For negative numbers, we use: encoded_value = i128::MAX + value + 1
/// This maps:
///   i128::MIN (-170141183460469231731687303715884105728) -> 0
///   -1                                                  -> i128::MAX (170141183460469231731687303715884105727)
///
/// This ensures that for any a < b (as signed integers), encode_i128(a) < encode_i128(b)
/// in lexicographic order.
fn encode_i128(i: i128) -> String {
    if i < 0 {
        // For negative numbers, we compute: i128::MAX + value + 1
        // where value is negative, so this equals i128::MAX - |value| + 1
        // Using wrapping arithmetic to handle i128::MIN correctly:
        // i128::MIN wraps to 0, -1 maps to i128::MAX
        let inverted = i.wrapping_add(i128::MAX).wrapping_add(1) as u128;
        format!("{}{}", NEGATIVE_PREFIX, encode_u128(inverted))
    } else {
        format!("{}{}", NON_NEGATIVE_PREFIX, encode_u128(i as u128))
    }
}

/// Decode a signed 128-bit integer from an ELEN-encoded string.
fn decode_i128(s: &str) -> Result<i128, ElenError> {
    if s.is_empty() {
        return Err(ElenError::InvalidSignedFormat(s.to_owned()));
    }

    let first_char = s.chars().next().unwrap();
    let rest = &s[1..];

    match first_char {
        NEGATIVE_PREFIX => {
            let inverted = decode_u128(rest)?;
            // Reverse the encoding: value = inverted - i128::MAX - 1
            // Using wrapping arithmetic to handle the full range
            let result = (inverted as i128).wrapping_sub(i128::MAX).wrapping_sub(1);
            Ok(result)
        }
        NON_NEGATIVE_PREFIX => {
            let value = decode_u128(rest)?;
            if value > i128::MAX as u128 {
                return Err(ElenError::InvalidSignedFormat(format!(
                    "value {} exceeds i128::MAX",
                    value
                )));
            }
            Ok(value as i128)
        }
        _ => Err(ElenError::InvalidSignedFormat(format!(
            "expected '{}' or '{}' prefix, got '{}'",
            NEGATIVE_PREFIX, NON_NEGATIVE_PREFIX, first_char
        ))),
    }
}

fn encode_u128(i: u128) -> String {
    let mut s = String::new();
    if i > 0 {
        s.push_str(LENGTH_RUNE);
    }
    let i_string = i.to_string();
    let i_string_length = i_string.len();
    if i_string_length > 1 {
        s.push_str(&encode_u128(i_string_length as u128));
    }
    s.push_str(&i_string);
    s
}

/// Slice `s[position .. position + len]`, returning `InvalidString` instead of **panicking** when the
/// range is out of bounds, its end overflows `usize`, or an endpoint is not on a UTF-8 char boundary.
///
/// ELEN decode advertises a `Result` contract, but a corrupt or crafted token (an empty or degenerate
/// length component) could make the raw `&s[..]` slices index out of bounds and panic *before* the
/// `?` could map the error — bypassing every caller's error handling (#250). `str::get` closes that.
fn elen_slice(s: &str, position: usize, len: u128) -> Result<&str, ElenError> {
    let len = usize::try_from(len).map_err(|_| ElenError::InvalidString(s.to_owned()))?;
    let end = position
        .checked_add(len)
        .ok_or_else(|| ElenError::InvalidString(s.to_owned()))?;
    s.get(position..end)
        .ok_or_else(|| ElenError::InvalidString(s.to_owned()))
}

fn decode_u128(s: &str) -> Result<u128, ElenError> {
    let total_elements: usize = s.chars().take_while(|&c| c == ';').count();
    let mut elements_remaining = total_elements;
    let mut position = total_elements;
    let mut next_parse_length: u128 = 1;
    while elements_remaining > 1 {
        let length_string = elen_slice(s, position, next_parse_length)?;
        next_parse_length = length_string
            .parse()
            .map_err(|_| ElenError::InvalidString(s.to_owned()))?;
        position += length_string.len();
        elements_remaining -= 1;
    }
    let element = elen_slice(s, position, next_parse_length)?;
    let result = element
        .parse()
        .map_err(|_| ElenError::InvalidString(s.to_owned()))?;
    Ok(result)
}

impl Elen {
    // pub fn encode_i8(i: i8) -> String {
    //     todo!()
    // }

    pub fn encode_u8(i: u8) -> Self {
        Self::encode_u128(i as u128)
    }

    // pub fn encode_i16(i: i16) -> Self {
    //     todo!()
    // }

    pub fn encode_u16(i: u16) -> Self {
        Self::encode_u128(i as u128)
    }

    // pub fn encode_i32(i: i32) -> Self {
    //     todo!()
    // }

    pub fn encode_u32(i: u32) -> Self {
        Self::encode_u128(i as u128)
    }

    pub fn encode_i64(i: i64) -> Self {
        Self(encode_i128(i as i128))
    }

    pub fn encode_u64(i: u64) -> Self {
        Self::encode_u128(i as u128)
    }

    // pub fn encode_i128(i: i128) -> Self {
    //     todo!()
    // }

    pub fn encode_u128(i: u128) -> Self {
        Self(encode_u128(i))
    }

    // pub fn encode_isize(i: isize) -> Self {
    //     todo!()
    // }

    pub fn encode_usize(i: usize) -> Self {
        Self::encode_u128(i as u128)
    }

    // pub fn decode_i8(&self) -> Result<i8, ElenError> {
    //     todo!()
    // }

    pub fn decode_u8(&self) -> Result<u8, ElenError> {
        decode_u128(&self.0)?
            .try_into()
            .map_err(ElenError::DecodeError)
    }

    // pub fn decode_i16(&self) -> Result<i16, ElenError> {
    //     todo!()
    // }

    pub fn decode_u16(&self) -> Result<u16, ElenError> {
        decode_u128(&self.0)?
            .try_into()
            .map_err(ElenError::DecodeError)
    }

    // pub fn decode_i32(&self) -> Result<i32, ElenError> {
    //     todo!()
    // }

    pub fn decode_u32(&self) -> Result<u32, ElenError> {
        decode_u128(&self.0)?
            .try_into()
            .map_err(ElenError::DecodeError)
    }

    pub fn decode_i64(&self) -> Result<i64, ElenError> {
        decode_i128(&self.0)?
            .try_into()
            .map_err(ElenError::DecodeError)
    }

    pub fn decode_u64(&self) -> Result<u64, ElenError> {
        decode_u128(&self.0)?
            .try_into()
            .map_err(ElenError::DecodeError)
    }

    // pub fn decode_i128(&self) -> Result<i128, ElenError> {
    //     todo!()
    // }

    pub fn decode_u128(&self) -> Result<u128, ElenError> {
        decode_u128(&self.0)
    }

    // pub fn decode_isize(&self) -> Result<isize, ElenError> {
    //     todo!()
    // }

    pub fn decode_usize(&self) -> Result<usize, ElenError> {
        decode_u128(&self.0)?
            .try_into()
            .map_err(ElenError::DecodeError)
    }
}

impl FromStr for Elen {
    type Err = ElenError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match decode_u128(s) {
            Ok(_) => Ok(Self(s.to_owned())),
            Err(e) => Err(e),
        }
    }
}

impl TryFrom<&str> for Elen {
    type Error = ElenError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Elen::from_str(value)
    }
}

impl Display for Elen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<Elen> for String {
    fn from(value: Elen) -> Self {
        value.0
    }
}

impl From<u8> for Elen {
    fn from(value: u8) -> Self {
        Elen::encode_u8(value)
    }
}

impl TryInto<u8> for Elen {
    type Error = ElenError;

    fn try_into(self) -> Result<u8, Self::Error> {
        self.decode_u8()
    }
}

impl From<u16> for Elen {
    fn from(value: u16) -> Self {
        Elen::encode_u16(value)
    }
}

impl TryInto<u16> for Elen {
    type Error = ElenError;

    fn try_into(self) -> Result<u16, Self::Error> {
        self.decode_u16()
    }
}

impl From<u32> for Elen {
    fn from(value: u32) -> Self {
        Elen::encode_u32(value)
    }
}

impl TryInto<u32> for Elen {
    type Error = ElenError;

    fn try_into(self) -> Result<u32, Self::Error> {
        self.decode_u32()
    }
}

impl From<u64> for Elen {
    fn from(value: u64) -> Self {
        Elen::encode_u64(value)
    }
}

impl TryInto<u64> for Elen {
    type Error = ElenError;

    fn try_into(self) -> Result<u64, Self::Error> {
        self.decode_u64()
    }
}

impl From<u128> for Elen {
    fn from(value: u128) -> Self {
        Elen::encode_u128(value)
    }
}

impl TryInto<u128> for Elen {
    type Error = ElenError;

    fn try_into(self) -> Result<u128, Self::Error> {
        self.decode_u128()
    }
}

impl From<usize> for Elen {
    fn from(value: usize) -> Self {
        Elen::encode_usize(value)
    }
}

impl TryInto<usize> for Elen {
    type Error = ElenError;

    fn try_into(self) -> Result<usize, Self::Error> {
        self.decode_usize()
    }
}

impl From<i64> for Elen {
    fn from(value: i64) -> Self {
        Elen::encode_i64(value)
    }
}

impl TryInto<i64> for Elen {
    type Error = ElenError;

    fn try_into(self) -> Result<i64, Self::Error> {
        self.decode_i64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let inputs: Vec<u128> = vec![
            0,
            1,
            999_999_999,
            1_000_000_000,
            123_123_123_123_123,
            123_123_123_123_123_123_123_123_123_123_123,
        ];
        for input in inputs {
            let s = encode_u128(input);
            let output = decode_u128(&s).unwrap();
            println!(
                "input: {}, encoded_string: {}, output: {}",
                input, s, output
            );
            assert_eq!(input, output);
        }
    }

    #[test]
    fn round_trip_signed() {
        let inputs: Vec<i128> = vec![
            i128::MIN,
            i64::MIN as i128,
            -1_000_000_000,
            -1,
            0,
            1,
            1_000_000_000,
            i64::MAX as i128,
            i128::MAX,
        ];
        for input in inputs {
            let s = encode_i128(input);
            let output = decode_i128(&s).unwrap();
            println!(
                "input: {}, encoded_string: {}, output: {}",
                input, s, output
            );
            assert_eq!(input, output);
        }
    }

    #[test]
    fn signed_ordering() {
        // Test that lexicographic ordering of encoded strings matches numeric ordering
        let values: Vec<i128> = vec![
            i128::MIN,
            i64::MIN as i128,
            -1_000_000_000,
            -100,
            -10,
            -1,
            0,
            1,
            10,
            100,
            1_000_000_000,
            i64::MAX as i128,
            i128::MAX,
        ];

        let encoded: Vec<String> = values.iter().map(|&v| encode_i128(v)).collect();

        // Verify that encoded strings maintain sort order
        for i in 0..encoded.len() - 1 {
            assert!(
                encoded[i] < encoded[i + 1],
                "Ordering violated: encode_i128({}) = '{}' should be < encode_i128({}) = '{}'",
                values[i],
                encoded[i],
                values[i + 1],
                encoded[i + 1]
            );
        }
    }

    #[test]
    fn i64_conversions() {
        let test_values: Vec<i64> = vec![i64::MIN, -1_000_000, -1, 0, 1, 1_000_000, i64::MAX];

        for input in test_values {
            let elen: Elen = input.into();
            let output: i64 = elen.try_into().unwrap();
            assert_eq!(input, output);
        }
    }

    #[test]
    fn conversions() {
        let input_u8: u8 = u8::MAX;
        let mut elen: Elen = input_u8.into();
        let mut output_u8: Result<u8, ElenError> = elen.try_into();
        assert_eq!(input_u8, output_u8.unwrap());

        let input_u16: u16 = u8::MAX as u16 + 1;
        elen = input_u16.into();
        output_u8 = elen.try_into();
        assert!(output_u8.is_err());
        elen = input_u16.into();
        let mut output_u16: Result<u16, ElenError> = elen.try_into();
        assert_eq!(input_u16, output_u16.unwrap());

        let input_u32: u32 = u16::MAX as u32 + 1;
        elen = input_u32.into();
        output_u16 = elen.try_into();
        assert!(output_u16.is_err());
        elen = input_u32.into();
        let mut output_u32: Result<u32, ElenError> = elen.try_into();
        assert_eq!(input_u32, output_u32.unwrap());

        let input_u64: u64 = u32::MAX as u64 + 1;
        elen = input_u64.into();
        output_u32 = elen.try_into();
        assert!(output_u32.is_err());
        elen = input_u64.into();
        let mut output_u64: Result<u64, ElenError> = elen.try_into();
        assert_eq!(input_u64, output_u64.unwrap());

        let input_usize: usize = u32::MAX as usize + 1;
        elen = input_usize.into();
        output_u32 = elen.try_into();
        assert!(output_u32.is_err());
        elen = input_usize.into();
        let mut output_usize: Result<usize, ElenError> = elen.try_into();
        assert_eq!(input_usize, output_usize.unwrap());

        let input_u128: u128 = u64::MAX as u128 + 1;
        elen = input_u128.into();
        output_u64 = elen.try_into();
        assert!(output_u64.is_err());
        elen = input_u128.into();
        output_usize = elen.try_into();
        assert!(output_usize.is_err());
        elen = input_u128.into();
        let output_u128: Result<u128, ElenError> = elen.try_into();
        assert_eq!(input_u128, output_u128.unwrap());
    }

    /// A malformed/degenerate ELEN token must return `Err`, never panic — the decoders index
    /// slices whose bounds come from the (untrusted) token, so a corrupt key on disk could otherwise
    /// unwind through `SlateDbKey::decode`'s `Result` contract (#250).
    #[test]
    fn malformed_tokens_error_not_panic() {
        // Empty, all-delimiter, and degenerate length prefixes (the reproductions from #250).
        // (";9" is intentionally absent — it is the *valid* encoding of 9.)
        for bad in ["", ";", ";;", ";;;", ";x", "!", "~", "!;", "~;"] {
            assert!(
                decode_u128(bad).is_err(),
                "decode_u128({bad:?}) must error, not panic"
            );
            // FromStr validates via decode_u128, so `parse::<Elen>()` (every key decode arm) is safe.
            assert!(
                bad.parse::<Elen>().is_err(),
                "{bad:?}.parse::<Elen>() must error, not panic"
            );
        }
        // Signed decode shares the same slicing (`decode_i128` → `decode_u128`).
        for bad in ["!", "~", "!!", "~;", "!;x"] {
            assert!(
                decode_i128(bad).is_err(),
                "decode_i128({bad:?}) must error, not panic"
            );
        }
    }

    /// A length prefix claiming an enormous element length must error, not panic or over-read
    /// (guards the `usize`/bounds arithmetic in `elen_slice`).
    #[test]
    fn oversized_length_prefix_errors() {
        // ";;9x": total_elements=2 ⇒ read a 1-char length "9", then try to read a 9-char element from
        // a 1-char remainder ("x") — must be an out-of-bounds error, not a panic.
        assert!(decode_u128(";;9x").is_err());
        assert!(decode_u128(";;99").is_err());
    }
}
