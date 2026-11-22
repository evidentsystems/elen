//! Properties of the integer encoding, over random values and strings.
#![cfg(feature = "alloc")]

use elen::{Decode, Elen, decode, encode};
use proptest::prelude::*;

/// Integers spread across every magnitude, not just near the extremes.
fn any_i128() -> impl Strategy<Value = i128> {
    prop_oneof![
        any::<i128>(),
        (0u32..128, any::<bool>()).prop_map(|(bits, neg)| {
            let m = (1i128 << (bits.min(126))) - 1;
            if neg { -m } else { m }
        }),
        -1000i128..1000,
    ]
}

/// Strings over ELEN's alphabet plus a few strays, so random input reaches deep into decoding.
fn elen_ish() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            8 => proptest::char::range('0', '9'),
            3 => Just(';'),
            3 => Just('-'),
            1 => proptest::sample::select(vec!['+', '!', '~', 'x', '\u{1f}', 'é']),
        ],
        0..48,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

macro_rules! round_trip {
    ($($name:ident: $t:ty),*) => {$(
        proptest! {
            #[test]
            fn $name(v: $t) {
                let s = encode(&v);
                prop_assert_eq!(s.len(), elen::Encode::encoded_len(&v));
                prop_assert_eq!(decode::<$t>(&s), Ok(v));
                // One number line: the same as the widest type's encoding.
                #[allow(irrefutable_let_patterns)]
                if let Ok(wide) = i128::try_from(v) {
                    prop_assert_eq!(&s, &encode(&wide));
                }
            }
        }
    )*};
}

round_trip!(
    round_trip_u8: u8, round_trip_u16: u16, round_trip_u32: u32, round_trip_u64: u64,
    round_trip_u128: u128, round_trip_usize: usize, round_trip_i8: i8, round_trip_i16: i16,
    round_trip_i32: i32, round_trip_i64: i64, round_trip_i128: i128, round_trip_isize: isize
);

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn order_is_preserved(a in any_i128(), b in any_i128()) {
        prop_assert_eq!(a.cmp(&b), encode(&a).cmp(&encode(&b)));
    }

    #[test]
    fn unsigned_order_is_preserved(a: u128, b: u128) {
        prop_assert_eq!(a.cmp(&b), encode(&a).cmp(&encode(&b)));
    }

    #[test]
    fn prefix_decoding_leaves_the_rest(v in any_i128(), rest in ".*") {
        let s = encode(&v) + &rest;
        prop_assert_eq!(i128::decode_prefix(&s), Ok((v, rest.as_str())));
    }

    #[test]
    fn tuples_sort_as_tuples(a in (any_i128(), any_i128()), b in (any_i128(), any_i128())) {
        prop_assert_eq!(a.cmp(&b), encode(&a).cmp(&encode(&b)));
    }

    /// The strictness guarantee: anything that decodes is the encoder's own spelling.
    #[test]
    fn only_canonical_spellings_decode(s in elen_ish()) {
        if let Ok(v) = decode::<i128>(&s) {
            prop_assert_eq!(encode(&v), s.clone());
        }
        if let Ok(v) = decode::<u8>(&s) {
            prop_assert_eq!(encode(&v), s.clone());
        }
        if let Ok(e) = s.parse::<Elen>() {
            prop_assert_eq!(e.as_str(), s.as_str());
        } else {
            // A string `Elen` refuses is no integer's encoding at all.
            prop_assert!(decode::<i128>(&s).is_err());
        }
        if let Ok((v, rest)) = i128::decode_prefix(&s) {
            prop_assert_eq!(encode(&v) + rest, s.clone());
        }
    }

    /// Mangling one byte of a valid encoding yields an error or another value's encoding.
    #[test]
    fn mangled_encodings_never_mislead(v in any_i128(), at: prop::sample::Index, c in elen_ish()) {
        let mut s = encode(&v).into_bytes();
        let i = at.index(s.len());
        let replacement = c.bytes().next().unwrap_or(b'0');
        s[i] = replacement;
        if let Ok(s) = String::from_utf8(s)
            && let Ok(w) = decode::<i128>(&s)
        {
            prop_assert_eq!(encode(&w), s);
        }
    }
}

/// Strings over the real line's alphabet.
fn real_ish() -> impl Strategy<Value = String> {
    prop_oneof![
        elen_ish(),
        Just("!".to_owned()),
        Just("~".to_owned()),
        (any::<f64>(), elen_ish()).prop_map(|(f, tail)| {
            let s = encode(&elen::NotNan::new(if f.is_nan() { 1.0 } else { f }).unwrap());
            // Keep a prefix of a valid encoding and append noise, to land near valid input.
            let cut = tail.len() % (s.len() + 1);
            format!("{}{}", &s[..cut], tail)
        }),
    ]
}

fn not_nan(f: f64) -> elen::NotNan<f64> {
    elen::NotNan::new(f).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn f64_round_trips_and_orders(a in any::<f64>().prop_filter("NaN", |f| !f.is_nan()),
                                  b in any::<f64>().prop_filter("NaN", |f| !f.is_nan())) {
        let (ea, eb) = (encode(&not_nan(a)), encode(&not_nan(b)));
        prop_assert_eq!(ea.len(), elen::Encode::encoded_len(&not_nan(a)));
        let back = decode::<f64>(&ea).unwrap();
        prop_assert!(back == a, "{} → {} → {}", a, ea, back);
        prop_assert_eq!(a.partial_cmp(&b).unwrap(), ea.cmp(&eb));
    }

    #[test]
    fn f32_round_trips_and_orders(a in any::<f32>().prop_filter("NaN", |f| !f.is_nan()),
                                  b in any::<f32>().prop_filter("NaN", |f| !f.is_nan())) {
        let (ea, eb) = (encode(&elen::NotNan::new(a).unwrap()), encode(&elen::NotNan::new(b).unwrap()));
        prop_assert!(decode::<f32>(&ea).unwrap() == a);
        prop_assert_eq!(a.partial_cmp(&b).unwrap(), ea.cmp(&eb));
        prop_assert!(elen::decode_rounded::<f32>(&ea).unwrap() == a);
    }

    /// The strictness guarantee on the real line.
    #[test]
    fn only_canonical_reals_decode(s in real_ish()) {
        if let Ok(v) = decode::<f64>(&s) {
            prop_assert_eq!(encode(&not_nan(v)), s.clone());
        }
        if let Ok(v) = decode::<f32>(&s) {
            prop_assert_eq!(encode(&elen::NotNan::new(v).unwrap()), s.clone());
        }
        if let Ok((v, rest)) = f64::decode_prefix(&s) {
            prop_assert_eq!(encode(&not_nan(v)) + rest, s.clone());
        }
    }
}

#[cfg(feature = "rust_decimal")]
mod rust_decimal_props {
    use elen::Fixed;
    use rust_decimal::Decimal;

    use super::*;

    fn any_decimal() -> impl Strategy<Value = Decimal> {
        prop_oneof![
            (any::<i64>(), 0u32..=28).prop_map(|(m, s)| Decimal::new(m, s)),
            (any::<i128>(), 0u32..=28).prop_filter_map("fits", |(m, s)| {
                Decimal::try_from_i128_with_scale(m >> 32, s).ok()
            }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(2048))]

        #[test]
        fn decimals_round_trip_and_order(a in any_decimal(), b in any_decimal()) {
            prop_assert_eq!(decode::<Decimal>(&encode(&a)), Ok(a));
            prop_assert_eq!(decode::<Fixed<Decimal>>(&encode(&Fixed(a))), Ok(Fixed(a)));
            prop_assert_eq!(a.cmp(&b), encode(&a).cmp(&encode(&b)));
            prop_assert_eq!(a.cmp(&b), encode(&Fixed(a)).cmp(&encode(&Fixed(b))));
            prop_assert_eq!(encode(&a).len(), elen::Encode::encoded_len(&a));
            prop_assert_eq!(encode(&Fixed(a)).len(), elen::Encode::encoded_len(&Fixed(a)));
        }

        /// Floats and decimals of the same value encode alike.
        #[test]
        fn floats_and_decimals_agree(m in -999_999_999_999_999i64..=999_999_999_999_999, scale in 0u32..=15) {
            let d = Decimal::new(m, scale);
            let f: f64 = d.to_string().parse().unwrap();
            prop_assert_eq!(encode(&d), encode(&not_nan(f)));
        }

        #[test]
        fn only_canonical_decimals_decode(s in real_ish()) {
            if let Ok(v) = decode::<Decimal>(&s) {
                prop_assert_eq!(encode(&v), s.clone());
            }
            if let Ok(v) = decode::<Fixed<Decimal>>(&s) {
                prop_assert_eq!(encode(&v), s.clone());
            }
        }
    }
}

#[cfg(feature = "bigdecimal")]
mod bigdecimal_props {
    use bigdecimal::{BigDecimal, num_bigint::BigInt};
    use elen::Fixed;

    use super::*;

    fn any_bigdecimal() -> impl Strategy<Value = BigDecimal> {
        (any::<i128>(), any::<i128>(), -60i64..60).prop_map(|(hi, lo, scale)| {
            let int = (BigInt::from(hi) << 128) + BigInt::from(lo);
            BigDecimal::new(int, scale)
        })
    }

    proptest! {
        #[test]
        fn bigdecimals_round_trip_and_order(a in any_bigdecimal(), b in any_bigdecimal()) {
            prop_assert_eq!(decode::<BigDecimal>(&encode(&a)), Ok(a.clone()));
            prop_assert_eq!(decode::<Fixed<BigDecimal>>(&encode(&Fixed(a.clone()))), Ok(Fixed(a.clone())));
            prop_assert_eq!(a.cmp(&b), encode(&a).cmp(&encode(&b)));
            prop_assert_eq!(a.cmp(&b), encode(&Fixed(a.clone())).cmp(&encode(&Fixed(b.clone()))));
        }

        #[test]
        fn only_canonical_bigdecimals_decode(s in real_ish()) {
            if let Ok(v) = decode::<BigDecimal>(&s) {
                prop_assert_eq!(encode(&v), s.clone());
            }
            if let Ok(v) = decode::<Fixed<BigDecimal>>(&s) {
                prop_assert_eq!(encode(&v), s.clone());
            }
        }
    }
}

#[cfg(feature = "num-bigint")]
mod bigint_props {
    use num_bigint::BigInt;

    use super::*;

    fn any_bigint() -> impl Strategy<Value = BigInt> {
        proptest::collection::vec(any::<i64>(), 0..6).prop_map(|limbs| {
            limbs
                .into_iter()
                .fold(BigInt::from(0), |acc, l| (acc << 64) + l)
        })
    }

    proptest! {
        #[test]
        fn bigints_round_trip_and_order(a in any_bigint(), b in any_bigint()) {
            prop_assert_eq!(decode::<BigInt>(&encode(&a)), Ok(a.clone()));
            prop_assert_eq!(a.cmp(&b), encode(&a).cmp(&encode(&b)));
            if let Ok(small) = i128::try_from(&a) {
                prop_assert_eq!(encode(&a), encode(&small));
            }
        }

        #[test]
        fn only_canonical_bigints_decode(s in elen_ish()) {
            if let Ok(v) = decode::<BigInt>(&s) {
                prop_assert_eq!(encode(&v), s.clone());
            }
        }
    }
}
