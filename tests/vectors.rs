//! Fixed examples: the paper's tables, extremes, and malformed input.
#![cfg(feature = "alloc")]

use elen::{Decode, Elen, Error, decode, encode};

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
fn paper_integers() {
    for &(n, s) in PAPER_INTEGERS {
        assert_eq!(encode(&n), s, "encode({n})");
        assert_eq!(decode::<i64>(s), Ok(n), "decode({s})");
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
            assert_eq!(encode(&v), extracted(v));
        }
        n = n.saturating_mul(10);
    }
    assert_eq!(encode(&u128::MAX), extracted(u128::MAX));
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
        assert_eq!(e.decode::<i128>(), Ok(*v));
    }
    assert!(encoded.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(Elen::from(u128::MAX).decode::<u128>(), Ok(u128::MAX));
    assert!(Elen::from(i128::MAX) < Elen::from(u128::MAX));
}

#[test]
fn exhaustive_16_bit() {
    let mut previous: Option<String> = None;
    for v in i16::MIN..=i16::MAX {
        let e = encode(&v);
        assert_eq!(e.len(), elen::Encode::encoded_len(&v));
        assert_eq!(decode::<i16>(&e), Ok(v));
        assert_eq!(e.parse::<Elen>().map(Elen::into_string).as_ref(), Ok(&e));
        if let Some(p) = previous {
            assert!(p < e, "{p} < {e}");
        }
        previous = Some(e);
    }
    for v in 0..=u16::MAX {
        assert_eq!(decode::<u16>(&encode(&v)), Ok(v));
    }
}

#[test]
fn narrowing_and_sign_errors() {
    assert_eq!(decode::<u8>(&encode(&256u16)), Err(Error::OutOfRange));
    assert_eq!(decode::<u64>(&encode(&-1i8)), Err(Error::OutOfRange));
    assert_eq!(decode::<i8>(&encode(&128i16)), Err(Error::OutOfRange));
    assert_eq!(decode::<i8>(&encode(&-128i16)), Ok(-128));
    assert_eq!(decode::<i8>(&encode(&-129i16)), Err(Error::OutOfRange));
    assert_eq!(decode::<i64>(&encode(&u64::MAX)), Err(Error::OutOfRange));
    let big = Elen::from(u128::MAX);
    assert_eq!(u64::try_from(&big), Err(Error::OutOfRange));
    assert_eq!(u128::try_from(big), Ok(u128::MAX));
}

/// Strings that are not exactly the encoder's spelling of a number must be refused, and never
/// panic.
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
        "!;5",     // the extracted encoder's signed prefixes
        "~;5",     //
        ";;2١٢",   // non-ASCII digits
        ";\u{e9}", // non-ASCII
    ] {
        assert!(bad.parse::<Elen>().is_err(), "{bad:?} must be refused");
        assert!(decode::<i128>(bad).is_err(), "{bad:?} as i128");
        assert!(decode::<u128>(bad).is_err(), "{bad:?} as u128");
        assert!(decode::<u8>(bad).is_err(), "{bad:?} as u8");
    }
}

#[test]
fn oversized_tokens_are_out_of_range_without_reading_them() {
    // Deeper than any u64 encoding, and a length longer than any u64 has digits.
    assert_eq!(decode::<u64>(";;;;2"), Err(Error::OutOfRange));
    assert_eq!(decode::<u64>(";;;299"), Err(Error::OutOfRange));
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
    // Positions in a tuple are relative to the whole input.
    assert_eq!(
        decode::<(u8, u8)>(";;212;;11"),
        Err(Error::NonCanonical { at: 7 })
    );
}

#[test]
fn tuples_and_prefixes() {
    let key = encode(&(42u64, -7i32, 0u8));
    assert_eq!(key, ";;242-20");
    assert_eq!(decode::<(u64, i32, u8)>(&key), Ok((42, -7, 0)));
    assert_eq!(
        decode::<(u64, i32)>(&key),
        Err(Error::TrailingInput { at: 7 })
    );
    assert_eq!(u64::decode_prefix(";;242\u{1f}x"), Ok((42, "\u{1f}x")));
    assert_eq!(<(u8, u8)>::decode_prefix(";1;2;3"), Ok(((1, 2), ";3")));
}

#[cfg(feature = "serde")]
#[test]
fn serde_round_trip() {
    let e = Elen::from(-10i64);
    let json = serde_json::to_string(&e).unwrap();
    assert_eq!(json, r#""--789""#);
    assert_eq!(serde_json::from_str::<Elen>(&json).unwrap(), e);
    assert!(serde_json::from_str::<Elen>(r#"";05""#).is_err());
}
