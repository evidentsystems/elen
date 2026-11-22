//! Efficient Lexicographic Encoding of Numbers (ELEN).
//!
//! Text encodings of numbers whose byte-wise lexicographic order is their numeric order, after
//! Peter Seymour's paper ["Efficient Lexicographic Encoding of Numbers"][paper]. Every integer
//! type shares one number line: `5u8`, `5u64` and `5i64` all encode as `;5`, and an encoding of
//! any integer sorts correctly against an encoding of any other.
//!
//! The paper writes its markers as `-` and `+`, ordered below and above the digits. ASCII `+`
//! sorts below the digits, so this crate writes [`POSITIVE`] (`;`) in its place:
//!
//! ```text
//! -1234567891 → ---7898765432108
//!         -10 → --789
//!          -1 → -8
//!           0 → 0
//!           1 → ;1
//!          10 → ;;210
//!  1234567890 → ;;;2101234567890
//! ```
//!
//! Decoding is strict: it accepts only the encoder's own spelling of each value, so a string
//! decodes successfully exactly when it is the encoding of the value it decodes to.
//!
//! Every encoding is self-delimiting. [`Decode::decode_prefix`] reads one from the front of a
//! longer string, and a tuple encodes as its fields' encodings concatenated, which sorts in the
//! tuple's order:
//!
//! ```
//! # #[cfg(feature = "alloc")] {
//! let key = elen::encode(&(42u64, -7i32));
//! assert_eq!(key, ";;242-2");
//! assert_eq!(elen::decode::<(u64, i32)>(&key)?, (42, -7));
//! assert!(elen::encode(&(42u64, -8i32)) < key);
//!
//! let (n, rest) = <u64 as elen::Decode>::decode_prefix(";;242/rest")?;
//! assert_eq!((n, rest), (42, "/rest"));
//! # }
//! # Ok::<(), elen::Error>(())
//! ```
//!
//! Every byte of an encoding is printable ASCII, from `!` (0x21) to `~` (0x7E), so any byte
//! below `!` works as a separator between an encoding and other text.
//!
//! # Features
//!
//! - `alloc` (default): [`Elen`], [`encode`], and the `String` conversions.
//! - `serde`: `Serialize` and `Deserialize` for [`Elen`], as a validated string.
//!
//! [paper]: https://www.zanopha.com/docs/elen.pdf
#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod elen;
mod error;
mod int;
mod primitive;
mod traits;

#[cfg(feature = "alloc")]
pub use elen::Elen;
pub use error::Error;
pub use traits::{Decode, Display, Encode, Integer};

/// Marks a positive number; sorts above every digit. The paper's `+`.
pub const POSITIVE: char = ';';
/// Marks a negative number; sorts below every digit. The paper's `-`.
pub const NEGATIVE: char = '-';
/// The encoding of zero.
pub const ZERO: char = '0';

/// Encodes a value.
#[cfg(feature = "alloc")]
pub fn encode<T: Encode + ?Sized>(value: &T) -> alloc::string::String {
    let mut s = alloc::string::String::with_capacity(value.encoded_len());
    value
        .encode_into(&mut s)
        .expect("writing to a String cannot fail");
    s
}

/// Decodes a value from `s`, which must hold exactly one encoding.
pub fn decode<T: Decode>(s: &str) -> Result<T, Error> {
    T::decode(s)
}

/// Formats a value as its encoding, without allocating.
///
/// ```
/// use core::fmt::Write;
///
/// let mut key = heapless_like::Buf::default();
/// write!(key, "seq/{}", elen::display(&10u32)).unwrap();
/// assert_eq!(key.as_str(), "seq/;;210");
/// # mod heapless_like {
/// #     #[derive(Default)]
/// #     pub struct Buf { bytes: [u8; 32], len: usize }
/// #     impl Buf { pub fn as_str(&self) -> &str { core::str::from_utf8(&self.bytes[..self.len]).unwrap() } }
/// #     impl core::fmt::Write for Buf {
/// #         fn write_str(&mut self, s: &str) -> core::fmt::Result {
/// #             let end = self.len + s.len();
/// #             self.bytes.get_mut(self.len..end).ok_or(core::fmt::Error)?.copy_from_slice(s.as_bytes());
/// #             self.len = end;
/// #             Ok(())
/// #         }
/// #     }
/// # }
/// ```
pub fn display<T: Encode + ?Sized>(value: &T) -> Display<'_, T> {
    Display(value)
}

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
