# elen

Efficient Lexicographic Encoding of Numbers (ELEN): text encodings of numbers whose byte-wise
lexicographic order matches numeric order, after Peter Seymour's paper
["Efficient Lexicographic Encoding of Numbers"](https://www.zanopha.com/docs/elen.pdf).

```rust
assert_eq!(elen::encode(&7u64), ";7");
assert_eq!(elen::encode(&10u64), ";;210");
assert_eq!(elen::encode(&-10i64), "--789");
assert!(elen::encode(&-10i64) < elen::encode(&7u8)); // one number line for every integer type

let key = elen::encode(&(42u64, -7i32)); // tuples concatenate and still sort as tuples
assert_eq!(elen::decode::<(u64, i32)>(&key)?, (42, -7));
# Ok::<(), elen::Error>(())
```

## When to use it

Use ELEN when an ordered number must be **text** that is compared as text:

- event sequence numbers (such as the CloudEvents `sequence` attribute),
- object-store keys, DynamoDB string sort keys, Redis keys, file names,
- keys you want to read in logs and JSON while debugging,
- numbers with no fixed size limit.

If the keys are bytes that nobody reads, a binary order-preserving encoding such as
[ordcode](https://docs.rs/ordcode), storekey or memcomparable is smaller and faster:

| value       | ELEN                          | bytes | ordcode bytes |
|-------------|-------------------------------|------:|--------------:|
| `0`         | `0`                           |     1 |             8 |
| `7`         | `;7`                          |     2 |             8 |
| `-7`        | `-2`                          |     2 |             8 |
| `1234567`   | `;;71234567`                  |    10 |             8 |
| `-1234567`  | `--28765432`                  |    10 |             8 |
| `i64::MAX`  | `;;;2199223372036854775807`   |    25 |             8 |
| `u64::MAX`  | `;;;22018446744073709551615`  |    26 |             8 |

Small numbers are short, and a number of `n` digits takes `n + O(log n)` bytes. Run
`cargo bench` to compare speeds on your machine.

## The encoding

A positive number is written as `k` markers, then a chain of lengths with the innermost first,
then its digits. Each length is prefixed only when it has more than one digit, and `k` counts
the groups after the markers. A negative number is its magnitude's encoding with negative
markers and every digit `d` replaced by `9 - d`. Zero is `0`.

The paper writes its markers as abstract symbols `-` < digits < `+`. ASCII `+` sorts *below*
the digits, so this crate uses `;` in its place:

| paper | this crate | ASCII |
|-------|------------|------:|
| `-`   | `-`        |  0x2D |
| `0-9` | `0-9`      | 0x30–0x39 |
| `+`   | `;`        |  0x3B |

```text
-1234567891 → ---7898765432108
        -10 → --789
         -1 → -8
          0 → 0
          1 → ;1
         10 → ;;210
 1234567890 → ;;;2101234567890
```

Every encoding is self-delimiting. `Decode::decode_prefix` reads one from the front of a longer
string, and concatenated encodings sort field by field. Every byte is printable ASCII, so any
byte below `!` (for example `\x1f`) works as a separator against surrounding text.

## Reals

Floats and decimals use the paper's §6 floating-point form. A real `±0.d₁…dₙ × 10ᵉ` is written
as a sign marker, then the exponent `e` as an integer (negated when the real is negative), then
the digits (inverted when negative), then a terminator. The terminator sorts below the digits
for positive reals and above them for negative ones. Zero is `0`. Infinities are `!` and `~`,
which sort below and above every other real.

```text
 -0.1×10¹¹ → ---7888;       0.00123 → ;-7123-
      -1.4 → --885;           0.123 → ;0123-
        -1 → --88;                1 → ;;11-
    -0.123 → -0876;             1.4 → ;;114-
         0 → 0             0.1×10¹¹ → ;;;2111-
```

`f32`, `f64`, `rust_decimal::Decimal` and `bigdecimal::BigDecimal` all write a value the same
way, so encodings of different real types compare correctly with each other:

```rust
# #[cfg(feature = "rust_decimal")] {
use elen::NotNan;
use rust_decimal::Decimal;

assert_eq!(elen::encode(&NotNan::new(-12.5f64)?), elen::encode(&Decimal::new(-125, 1)));
# }
# Ok::<(), elen::Error>(())
```

- **NaN** has no place in numeric order, so floats are encoded through `NotNan`, which refuses
  NaN. `-0.0` encodes as `0`.
- **Exactness.** Decoding refuses a number the target type cannot hold exactly. For example, a
  30-digit decimal can't be read as an `f64`, and `1e30` can't be read as a `Decimal`.
  `elen::decode_rounded::<f64>` rounds to the nearest float instead.
- **Fixed-point.** `Fixed<Decimal>` and `Fixed<BigDecimal>` use the paper's §5 form instead: the
  integer part, then the fraction. `100.5` is `;;31005-`. These encodings form their own line.
  §5 is longer for extreme magnitudes but reads like an ordinary decimal.
- **Paper errata.** The paper's §5 table leaves a one-digit integer part uninverted in
  negative numbers, for example `-3.14 → -385+`. Those rows would sort out of order (`-385+`
  sorts after `-198+`, which the paper gives for `-1.01`). This crate inverts every digit of a
  negative number, as the paper's §3 and its multi-digit §5 rows do: `-3.14 → -685;`.

Integers and reals are different number lines: `1u8` is `;1`, while `1.0` is `;;11-`.

## Strictness

Decoding accepts only the encoder's own spelling of each value. If `T::decode(s)` returns
`Ok(v)`, then `encode(&v) == s`. Decoding refuses:

- trailing input (`;7junk`), unless you ask for a prefix,
- leading zeros (`;;205`), redundant or zero length prefixes (`;;11`, `;;05`), and a signed zero
  (`;0`, `-9`),
- any character other than the markers and ASCII digits (`;;2+5`),
- encodings deeper or longer than the target type can hold. These fail with
  `Error::OutOfRange` before the digits are read.

If equal numbers could have different spellings, equality lookups and range scans over encoded
keys would silently miss data. Strict decoding rules that out. Property tests and fuzz targets check
it, for every type.

## Features

- `alloc` (default): `Elen`, `encode`, `decode_rounded`, and `String` conversions. Without it
  the crate is `no_std` with no allocation: use `Encode::encode_into` or `elen::display`.
  Primitive integers and floats work either way.
- `serde`: `Serialize` and `Deserialize` for `Elen`, as a validated string.
- `num-bigint`: `BigInt` and `BigUint` on the integer line.
- `rust_decimal`: `Decimal` on the real line and as `Fixed<Decimal>`.
- `bigdecimal`: `BigDecimal` on the real line and as `Fixed<BigDecimal>`.

Decoding a bigint or `BigDecimal` checks the input in linear time but then parses it in
superlinear time, so limit the length of untrusted input.

The minimum supported Rust version is 1.88.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
