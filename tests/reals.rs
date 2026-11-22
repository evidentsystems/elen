//! The real line and fixed-point decimals: the paper's tables, specials, and malformed input.
#![cfg(feature = "alloc")]

use elen::{Decode, Error, NotNan, decode, decode_rounded, encode};

/// The floating-point table from §6 of the paper, with `+` written as `;` and the spaces after
/// the sign removed.
const PAPER_REALS: &[(f64, &str)] = &[
    (-1e10, "---7888;"),
    (-1e9, "---7898;"),
    (-1.4, "--885;"),
    (-1.3, "--886;"),
    (-1.0, "--88;"),
    (-0.123, "-0876;"),
    (-0.0123, "-;1876;"),
    (-0.001233, "-;28766;"),
    (-0.00123, "-;2876;"),
    (0.0, "0"),
    (0.00123, ";-7123-"),
    (0.001233, ";-71233-"),
    (0.0123, ";-8123-"),
    (0.123, ";0123-"),
    (1.0, ";;11-"),
    (1.3, ";;113-"),
    (1.4, ";;114-"),
    (1e9, ";;;2101-"),
    (1e10, ";;;2111-"),
];

/// The large-decimal table from §5 of the paper, with `+` written as `;`.
///
/// The paper's rows for numbers whose integer part has one digit leave that digit uninverted
/// (`-3.14` as `-385+`, `-1` as `-1+`, `-0.000123` as `-0999876+`), which would sort them in the
/// wrong order (`-385+` sorts above `-198+`, the paper's `-1.01`). The rows below invert every
/// digit of a negative number, as the paper's §3 and its multi-digit rows (`-10.5` as `--7894+`)
/// do.
#[cfg(any(feature = "rust_decimal", feature = "bigdecimal"))]
const PAPER_FIXED: &[(&str, &str)] = &[
    ("-100.5", "--68994;"),
    ("-10.5", "--7894;"),
    ("-3.145", "-6854;"),
    ("-3.14", "-685;"),
    ("-1.01", "-898;"),
    ("-1", "-8;"),
    ("-0.0001233", "-99998766;"),
    ("-0.000123", "-9999876;"),
    ("0", "0"),
    ("0.000123", ";0000123-"),
    ("0.0001233", ";00001233-"),
    ("1", ";1-"),
    ("1.01", ";101-"),
    ("3.14", ";314-"),
    ("3.145", ";3145-"),
    ("10.5", ";;2105-"),
    ("100.5", ";;31005-"),
];

fn ascending(encodings: impl IntoIterator<Item = String>) {
    let encodings: Vec<String> = encodings.into_iter().collect();
    for pair in encodings.windows(2) {
        assert!(pair[0] < pair[1], "{} < {}", pair[0], pair[1]);
    }
}

#[test]
fn paper_reals_as_floats() {
    for &(v, s) in PAPER_REALS {
        assert_eq!(encode(&NotNan::new(v).unwrap()), s, "encode({v})");
        assert_eq!(decode::<f64>(s), Ok(v), "decode({s})");
    }
    ascending(PAPER_REALS.iter().map(|&(_, s)| s.to_owned()));
}

#[test]
fn specials() {
    assert_eq!(encode(&NotNan::new(f64::NEG_INFINITY).unwrap()), "!");
    assert_eq!(encode(&NotNan::new(f64::INFINITY).unwrap()), "~");
    assert_eq!(encode(&NotNan::new(-0.0f64).unwrap()), "0");
    assert_eq!(decode::<f64>("!"), Ok(f64::NEG_INFINITY));
    assert_eq!(decode::<f32>("~"), Ok(f32::INFINITY));
    assert_eq!(NotNan::new(f32::NAN), Err(Error::NaN));
    ascending(
        [
            f64::NEG_INFINITY,
            f64::MIN,
            -1.0,
            -f64::MIN_POSITIVE,
            -5e-324,
            0.0,
            5e-324,
            f64::MIN_POSITIVE,
            1.0,
            f64::MAX,
            f64::INFINITY,
        ]
        .map(|v| encode(&NotNan::new(v).unwrap())),
    );
}

#[test]
fn floats_hold_only_what_they_hold_exactly() {
    // 0.1 is the shortest form of the f64 nearest 0.1, but not of any f32-exact value's f64.
    let tenth = encode(&NotNan::new(0.1f64).unwrap());
    assert_eq!(decode::<f64>(&tenth), Ok(0.1));
    assert_eq!(decode::<f32>(&tenth), Ok(0.1f32));
    let f64_only = encode(&NotNan::new(0.1f64 + 0.2).unwrap()); // 0.30000000000000004
    assert_eq!(decode::<f32>(&f64_only), Err(Error::OutOfRange));
    // Beyond f64's range, and more digits than an f64 keeps.
    assert_eq!(decode::<f64>(";;;34001-"), Err(Error::OutOfRange));
    let long = ";0333333333333333333333333333-";
    assert_eq!(decode::<f64>(long), Err(Error::OutOfRange));
    assert_eq!(decode_rounded::<f64>(long), Ok(1.0 / 3.0));
    assert_eq!(decode_rounded::<f64>(";;;34001-"), Ok(f64::INFINITY));
    assert_eq!(decode_rounded::<f32>(&f64_only), Ok(0.3f32));
}

#[test]
fn malformed_reals_error() {
    for bad in [
        "", ";", ";1", ";1-x", ";01",     // truncated or trailing
        ";0-",     // no mantissa digits
        ";00123-", // a leading zero in the mantissa
        ";01230-", // a trailing zero in the mantissa
        ";0123;",  // a positive number's terminator is `-`
        "-0876-",  // a negative number's terminator is `;`
        ";;0123-", // a non-canonical exponent
        "!!", "~0", "!;", "+1", ";;11+",
    ] {
        assert!(decode::<f64>(bad).is_err(), "{bad:?} as f64");
        assert!(decode_rounded::<f64>(bad).is_err(), "{bad:?} rounded");
    }
}

#[test]
fn integers_and_reals_are_different_lines() {
    assert_eq!(encode(&1u8), ";1");
    assert_eq!(encode(&NotNan::new(1.0f64).unwrap()), ";;11-");
    assert!(decode::<f64>(";1").is_err());
    assert!(decode::<u8>(";;11-").is_err());
}

#[test]
fn tuples_mix_lines() {
    let key = encode(&(3u8, NotNan::new(-0.5f64).unwrap(), 4i8));
    assert_eq!(decode::<(u8, f64, i8)>(&key), Ok((3, -0.5, 4)));
    let (v, rest) = f64::decode_prefix(";0123-\u{1f}tail").unwrap();
    assert_eq!((v, rest), (0.123, "\u{1f}tail"));
}

#[cfg(feature = "rust_decimal")]
mod rust_decimal_tests {
    use std::str::FromStr;

    use elen::Fixed;
    use rust_decimal::Decimal;

    use super::*;

    #[test]
    fn paper_reals() {
        for &(v, s) in PAPER_REALS {
            let d = Decimal::from_str(&format!("{v}")).unwrap();
            assert_eq!(encode(&d), s, "encode({d})");
            assert_eq!(decode::<Decimal>(s), Ok(d), "decode({s})");
        }
    }

    #[test]
    fn paper_fixed() {
        for &(v, s) in PAPER_FIXED {
            let d = Decimal::from_str(v).unwrap();
            assert_eq!(encode(&Fixed(d)), s, "encode({d})");
            assert_eq!(decode::<Fixed<Decimal>>(s), Ok(Fixed(d)), "decode({s})");
        }
        ascending(PAPER_FIXED.iter().map(|&(_, s)| s.to_owned()));
    }

    #[test]
    fn scale_and_trailing_zeros_do_not_matter() {
        let a = Decimal::from_str("1.500").unwrap();
        let b = Decimal::from_str("1.5").unwrap();
        assert_eq!(encode(&a), encode(&b));
        assert_eq!(encode(&Decimal::from_str("100").unwrap()), ";;31-");
        assert_eq!(encode(&Fixed(Decimal::from_str("100").unwrap())), ";;3100-");
        assert_eq!(encode(&Decimal::from_str("-0.00").unwrap()), "0");
    }

    #[test]
    fn extremes() {
        for d in [
            Decimal::MAX,
            Decimal::MIN,
            Decimal::new(1, 28),
            Decimal::new(-1, 28),
        ] {
            assert_eq!(decode::<Decimal>(&encode(&d)), Ok(d));
            assert_eq!(decode::<Fixed<Decimal>>(&encode(&Fixed(d))), Ok(Fixed(d)));
        }
        // Too many digits, too small, and too large.
        assert_eq!(
            decode::<Decimal>(&encode(&NotNan::new(1.0 / 3.0).unwrap())),
            Ok(Decimal::from_str("0.3333333333333333").unwrap())
        );
        assert_eq!(decode::<Decimal>(";--7701-"), Err(Error::OutOfRange)); // 0.1×10⁻²⁹
        assert_eq!(decode::<Decimal>(";;;2301-"), Err(Error::OutOfRange)); // 0.1×10³⁰
        assert_eq!(decode::<Decimal>("~"), Err(Error::OutOfRange));
    }

    #[test]
    fn floats_and_decimals_share_a_line() {
        for s in ["-12.5", "-0.001", "0.1", "0.3", "2", "1234.5678", "1e20"] {
            let f: f64 = s.parse().unwrap();
            let d = Decimal::from_scientific(s)
                .or_else(|_| Decimal::from_str(s))
                .unwrap();
            assert_eq!(encode(&NotNan::new(f).unwrap()), encode(&d), "{s}");
        }
    }
}

#[cfg(feature = "bigdecimal")]
mod bigdecimal_tests {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;
    use elen::Fixed;

    use super::*;

    #[test]
    fn paper_reals() {
        for &(v, s) in PAPER_REALS {
            let d = BigDecimal::from_str(&format!("{v}")).unwrap();
            assert_eq!(encode(&d), s, "encode({d})");
            assert_eq!(decode::<BigDecimal>(s), Ok(d), "decode({s})");
        }
    }

    #[test]
    fn paper_fixed() {
        for &(v, s) in PAPER_FIXED {
            let d = BigDecimal::from_str(v).unwrap();
            assert_eq!(encode(&Fixed(d.clone())), s, "encode({d})");
            assert_eq!(decode::<Fixed<BigDecimal>>(s), Ok(Fixed(d)), "decode({s})");
        }
    }

    #[test]
    fn huge_and_tiny() {
        for s in [
            "1e1000",
            "-1.5e-1000",
            "123456789012345678901234567890.123456789",
        ] {
            let d = BigDecimal::from_str(s).unwrap();
            assert_eq!(decode::<BigDecimal>(&encode(&d)), Ok(d.clone()), "{s}");
        }
        assert!(encode(&BigDecimal::from_str("1e1000").unwrap()).as_str() < "~");
        assert_eq!(
            encode(&BigDecimal::from_str("1e1000").unwrap()),
            ";;;410011-"
        );
    }
}

#[cfg(feature = "num-bigint")]
mod bigint_tests {
    use elen::Elen;
    use num_bigint::{BigInt, BigUint};

    use super::*;

    #[test]
    fn bigints_share_the_integer_line() {
        for v in [i128::MIN, -10, -1, 0, 1, 10, i128::MAX] {
            let b = BigInt::from(v);
            assert_eq!(encode(&b), encode(&v));
            assert_eq!(decode::<BigInt>(&encode(&v)), Ok(b));
        }
        let huge: BigInt = BigInt::from(10).pow(1000) * -7;
        let e = Elen::from(&huge);
        assert_eq!(e.decode::<BigInt>(), Ok(huge.clone()));
        assert_eq!(e.decode::<i128>(), Err(Error::OutOfRange));
        assert_eq!(decode::<BigUint>(e.as_str()), Err(Error::OutOfRange));
        assert!(e < Elen::from(i128::MIN));
        assert_eq!(decode::<BigUint>(";;212"), Ok(BigUint::from(12u8)));
        assert!(decode::<BigInt>(";;205").is_err());
    }
}
