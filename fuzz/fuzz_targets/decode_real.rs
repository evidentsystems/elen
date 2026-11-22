//! Decoding arbitrary input as a real or a bigint never panics, and anything that decodes is the
//! encoder's spelling.
#![no_main]

use bigdecimal::BigDecimal;
use elen::{Decode, Fixed, NotNan, decode, decode_rounded, encode};
use libfuzzer_sys::fuzz_target;
use num_bigint::BigInt;
use rust_decimal::Decimal;

fuzz_target!(|s: &str| {
    // Keep bigint and bigdecimal parsing, which is superlinear, to short inputs.
    let short = s.len() <= 256;
    if let Ok(v) = decode::<f64>(s) {
        assert_eq!(encode(&NotNan::new(v).unwrap()), s);
    }
    if let Ok(v) = decode::<f32>(s) {
        assert_eq!(encode(&NotNan::new(v).unwrap()), s);
    }
    let _ = decode_rounded::<f64>(s);
    if let Ok(v) = decode::<Decimal>(s) {
        assert_eq!(encode(&v), s);
    }
    if let Ok(v) = decode::<Fixed<Decimal>>(s) {
        assert_eq!(encode(&v), s);
    }
    if short {
        if let Ok(v) = decode::<BigDecimal>(s) {
            assert_eq!(encode(&v), s);
        }
        if let Ok(v) = decode::<Fixed<BigDecimal>>(s) {
            assert_eq!(encode(&v), s);
        }
        if let Ok(v) = decode::<BigInt>(s) {
            assert_eq!(encode(&v), s);
        }
    }
    if let Ok((v, rest)) = <(f64, Decimal)>::decode_prefix(s) {
        assert_eq!(encode(&(NotNan::new(v.0).unwrap(), v.1)) + rest, s);
    }
});
