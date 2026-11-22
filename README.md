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
keys would silently miss data. This rule rules that out. Property tests and a fuzz target check
it.

## Features

- `alloc` (default): `Elen`, `encode`, and `String` conversions. Without it the crate is
  `no_std` with no allocation: use `Encode::encode_into` or `elen::display`.
- `serde`: `Serialize` and `Deserialize` for `Elen`, as a validated string.

The minimum supported Rust version is 1.88.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
