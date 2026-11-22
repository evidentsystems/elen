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
