# Changelog

## Unreleased

First release, extracted from `evidentsource-domain`'s `elen` module.

- Integers follow the paper's §3. Negatives are digit-inverted, so `-1` is `-8`, and every
  integer type shares one number line. Non-negative encodings are unchanged from the extracted
  module. Signed encodings changed: the old `!`/`~` prefixes are gone.
- Decoding is strict and accepts only canonical spellings.
- `Encode`/`Decode` traits for every primitive integer and for tuples of up to 8 fields, with
  `decode_prefix` for reading from the front of a string.
- `Elen`, a validated encoding of an integer of any size.
- `no_std`, with an optional `alloc` (default) and an optional `serde` feature.
- Reals on the paper's §6 line: `f32`/`f64` through `NotNan`, and `rust_decimal::Decimal` and
  `bigdecimal::BigDecimal` behind features. All of them encode a value the same way. Infinities
  sort at the ends, and NaN is refused. Decoding is exact; `decode_rounded` rounds.
- `Fixed<T>`, the paper's §5 fixed-point form, for both decimal types. Every digit of a negative
  number is inverted, which corrects the paper's table for one-digit integer parts.
- `BigInt` and `BigUint` on the integer line, behind the `num-bigint` feature.
