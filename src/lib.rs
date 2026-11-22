//! Efficient Lexicographic Encoding of Numbers (ELEN).
//!
//! Text encodings of numbers whose byte-wise lexicographic order is their numeric order, after
//! Peter Seymour's paper ["Efficient Lexicographic Encoding of Numbers"][paper]. Every integer
//! type shares one number line: `5u8`, `5u64` and `5i64` all encode as `;5`, and an encoding of
//! any integer sorts correctly against an encoding of any other.
//!
//! The paper writes its markers as `-` and `+`, ordered below and above the digits. ASCII `+`
//! sorts below the digits, so this crate writes [`POSITIVE`] (`;`) in its place:
//!
//! ```text
//! -1234567891 → ---7898765432108
//!         -10 → --789
//!          -1 → -8
//!           0 → 0
//!           1 → ;1
//!          10 → ;;210
//!  1234567890 → ;;;2101234567890
//! ```
//!
//! Decoding is strict: it accepts only the encoder's own spelling of each number, so a string
//! decodes successfully exactly when it is the encoding of the value it decodes to.
//!
//! [paper]: https://www.zanopha.com/docs/elen.pdf

use std::{fmt::Display, str::FromStr};

mod error;
mod int;

pub use error::Error;

/// The error type's former name.
pub type ElenError = Error;

/// Marks a positive number; sorts above every digit. The paper's `+`.
pub const POSITIVE: char = ';';
/// Marks a negative number; sorts below every digit. The paper's `-`.
pub const NEGATIVE: char = '-';
/// The encoding of zero.
pub const ZERO: char = '0';

/// A validated ELEN encoding of an integer.
///
/// `Elen`s compare in numeric order, since their bytes do.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Elen(String);

macro_rules! primitive {
    ($($t:ty => $encode:ident, $decode:ident;)*) => {
        impl Elen {
            $(
                #[doc = concat!("Encodes a `", stringify!($t), "`.")]
                pub fn $encode(i: $t) -> Self {
                    let mut s = String::with_capacity(int::primitive_encoded_len(i));
                    int::encode_primitive(i, &mut s).expect("writing to a String cannot fail");
                    Self(s)
                }

                #[doc = concat!("Decodes a `", stringify!($t), "`, failing with [`Error::OutOfRange`] when the number does not fit.")]
                pub fn $decode(&self) -> Result<$t, Error> {
                    int::finish(&self.0, int::decode_primitive_prefix(&self.0)?)
                }
            )*
        }

        $(
            impl From<$t> for Elen {
                fn from(value: $t) -> Self {
                    Elen::$encode(value)
                }
            }

            impl TryFrom<&Elen> for $t {
                type Error = Error;

                fn try_from(value: &Elen) -> Result<Self, Self::Error> {
                    value.$decode()
                }
            }

            impl TryFrom<Elen> for $t {
                type Error = Error;

                fn try_from(value: Elen) -> Result<Self, Self::Error> {
                    value.$decode()
                }
            }
        )*
    };
}

primitive! {
    u8 => encode_u8, decode_u8;
    u16 => encode_u16, decode_u16;
    u32 => encode_u32, decode_u32;
    u64 => encode_u64, decode_u64;
    u128 => encode_u128, decode_u128;
    usize => encode_usize, decode_usize;
    i8 => encode_i8, decode_i8;
    i16 => encode_i16, decode_i16;
    i32 => encode_i32, decode_i32;
    i64 => encode_i64, decode_i64;
    i128 => encode_i128, decode_i128;
    isize => encode_isize, decode_isize;
}

impl Elen {
    /// The encoding.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Elen {
    type Err = Error;

    /// Validates that `s` is exactly the encoding of some integer, of any size.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let token = int::read(s, usize::MAX)?;
        int::finish(s, ((), &s[token.end..]))?;
        Ok(Self(s.to_owned()))
    }
}

impl TryFrom<&str> for Elen {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The integer table from §3 of the paper, with `+` written as `;`.
    const PAPER_INTEGERS: &[(i64, &str)] = &[
        (-1234567891, "---7898765432108"),
        (-1234567890, "---7898765432109"),
        (-1234567889, "---7898765432110"),
        (-11, "--788"),
        (-10, "--789"),
        (-9, "-0"),
        (-2, "-7"),
        (-1, "-8"),
        (0, "0"),
        (1, ";1"),
        (2, ";2"),
        (9, ";9"),
        (10, ";;210"),
        (11, ";;211"),
        (1234567889, ";;;2101234567889"),
        (1234567890, ";;;2101234567890"),
        (1234567891, ";;;2101234567891"),
    ];

    #[test]
    fn paper_vectors() {
        for &(n, s) in PAPER_INTEGERS {
            assert_eq!(Elen::from(n).as_str(), s, "encode({n})");
            assert_eq!(
                s.parse::<Elen>().unwrap().decode_i64().unwrap(),
                n,
                "decode({s})"
            );
        }
        for pair in PAPER_INTEGERS.windows(2) {
            assert!(pair[0].1 < pair[1].1, "{} < {}", pair[0].1, pair[1].1);
        }
    }

    /// Non-negative numbers keep the spelling of the encoder this crate was extracted from, so
    /// unsigned encodings already stored stay valid.
    #[test]
    fn unsigned_encodings_match_the_extracted_encoder() {
        fn extracted(i: u128) -> String {
            let mut s = String::new();
            if i > 0 {
                s.push(';');
            }
            let digits = i.to_string();
            if digits.len() > 1 {
                s.push_str(&extracted(digits.len() as u128));
            }
            s.push_str(&digits);
            s
        }
        let mut n = 1u128;
        for _ in 0..39 {
            for v in [n - 1, n, n + 1, n.saturating_mul(7) / 3] {
                assert_eq!(Elen::from(v).as_str(), extracted(v));
            }
            n = n.saturating_mul(10);
        }
        assert_eq!(Elen::from(u128::MAX).as_str(), extracted(u128::MAX));
    }

    #[test]
    fn every_type_shares_one_number_line() {
        assert_eq!(Elen::from(5u8), Elen::from(5i64));
        assert_eq!(Elen::from(5u128), Elen::from(5isize));
        assert_eq!(Elen::from(-5i8), Elen::from(-5i128));
        assert_eq!(Elen::from(u64::MAX), Elen::from(u64::MAX as i128));
    }

    #[test]
    fn extremes_round_trip_and_order() {
        let values = [
            i128::MIN,
            i128::MIN + 1,
            i64::MIN as i128,
            -1_000_000_000,
            -1,
            0,
            1,
            1_000_000_000,
            i64::MAX as i128,
            i128::MAX,
        ];
        let encoded: Vec<Elen> = values.iter().map(|&v| Elen::from(v)).collect();
        for (v, e) in values.iter().zip(&encoded) {
            assert_eq!(e.decode_i128().unwrap(), *v);
        }
        assert!(encoded.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(Elen::from(u128::MAX).decode_u128().unwrap(), u128::MAX);
        assert!(Elen::from(i128::MAX) < Elen::from(u128::MAX));
    }

    #[test]
    fn exhaustive_16_bit() {
        let mut previous: Option<Elen> = None;
        for v in i16::MIN..=i16::MAX {
            let e = Elen::from(v);
            assert_eq!(e.decode_i16().unwrap(), v);
            assert_eq!(e.as_str().parse::<Elen>().unwrap(), e);
            if let Some(p) = previous {
                assert!(p < e, "{p} < {e}");
            }
            previous = Some(e);
        }
        for v in 0..=u16::MAX {
            assert_eq!(Elen::from(v).decode_u16().unwrap(), v);
        }
    }

    #[test]
    fn narrowing_and_sign_errors() {
        assert_eq!(Elen::from(256u16).decode_u8(), Err(Error::OutOfRange));
        assert_eq!(Elen::from(-1i8).decode_u64(), Err(Error::OutOfRange));
        assert_eq!(Elen::from(128i16).decode_i8(), Err(Error::OutOfRange));
        assert_eq!(Elen::from(-128i16).decode_i8(), Ok(-128));
        assert_eq!(Elen::from(-129i16).decode_i8(), Err(Error::OutOfRange));
        assert_eq!(Elen::from(u64::MAX).decode_i64(), Err(Error::OutOfRange));
        assert_eq!(
            Elen::from(u32::MAX as u64 + 1).decode_u32(),
            Err(Error::OutOfRange)
        );
        let big: Elen = Elen::from(u128::MAX);
        assert_eq!(u64::try_from(&big), Err(Error::OutOfRange));
        assert_eq!(u128::try_from(big), Ok(u128::MAX));
    }

    /// Strings that are not exactly the encoder's spelling of a number must be refused, and
    /// never panic.
    #[test]
    fn malformed_and_non_canonical_tokens_error() {
        for bad in [
            "",        // empty
            ";",       // markers only
            ";;",      //
            ";;;",     //
            "-",       //
            ";x",      // not a digit
            "5",       // no marker
            ";05",     // zero magnitude, then trailing
            ";0",      // zero is `0`
            "-9",      // `-9` is the negated zero
            "5xyz",    //
            ";;2+1",   // `+` is not a digit, even though `u128::from_str` takes it
            ";;11",    // a length of 1 is never prefixed
            ";;05",    // a zero length
            ";;9x",    // truncated
            ";;99",    // truncated
            ";;212x",  // trailing input
            "00",      // trailing input after zero
            ";-1",     // mixed markers
            "-;1",     //
            "!;5",     // the old signed prefixes
            "~;5",     //
            ";;2١٢",   // non-ASCII digits
            ";\u{e9}", // non-ASCII
        ] {
            assert!(bad.parse::<Elen>().is_err(), "{bad:?} must be refused");
            assert!(
                Elen(bad.to_owned()).decode_i128().is_err(),
                "decode_i128({bad:?}) must error"
            );
            assert!(
                Elen(bad.to_owned()).decode_u128().is_err(),
                "decode_u128({bad:?}) must error"
            );
        }
    }

    #[test]
    fn oversized_tokens_are_out_of_range_without_reading_them() {
        // Deeper than any u64 encoding, and a length longer than any u64 has digits.
        assert_eq!(Elen(";;;;2".into()).decode_u64(), Err(Error::OutOfRange));
        assert_eq!(Elen(";;;299".into()).decode_u64(), Err(Error::OutOfRange));
        // A length that overflows `usize`.
        let huge = format!(";;;;299{}", "9".repeat(99));
        assert!(huge.parse::<Elen>().is_err());
    }

    #[test]
    fn error_positions() {
        assert_eq!(
            ";;2+1".parse::<Elen>(),
            Err(Error::UnexpectedChar { at: 3, found: '+' })
        );
        assert_eq!(
            ";;212x".parse::<Elen>(),
            Err(Error::TrailingInput { at: 5 })
        );
        assert_eq!(";;11".parse::<Elen>(), Err(Error::NonCanonical { at: 2 }));
        assert_eq!(";;99".parse::<Elen>(), Err(Error::UnexpectedEnd));
        assert_eq!("".parse::<Elen>(), Err(Error::Empty));
    }
}
