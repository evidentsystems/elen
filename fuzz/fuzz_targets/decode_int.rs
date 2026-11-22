//! Decoding arbitrary input never panics, and anything that decodes is the encoder's spelling.
#![no_main]

use elen::{Decode, Elen, decode, encode};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|s: &str| {
    if let Ok(v) = decode::<i128>(s) {
        assert_eq!(encode(&v), s);
    }
    if let Ok(v) = decode::<u128>(s) {
        assert_eq!(encode(&v), s);
    }
    if let Ok(v) = decode::<i8>(s) {
        assert_eq!(encode(&v), s);
    }
    if let Ok((v, rest)) = <(u64, i64)>::decode_prefix(s) {
        assert_eq!(encode(&v) + rest, s);
    }
    match s.parse::<Elen>() {
        Ok(e) => assert_eq!(e.as_str(), s),
        Err(_) => assert!(decode::<i128>(s).is_err() && decode::<u128>(s).is_err()),
    }
});
