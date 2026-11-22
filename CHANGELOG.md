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
