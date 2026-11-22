use core::fmt;

/// Why a string is not a valid ELEN encoding of the requested type, or why a value cannot be
/// encoded.
///
/// Positions (`at`) are byte offsets into the string handed to the decoder.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Error {
    /// The input was empty.
    Empty,
    /// A character that cannot appear at this position.
    UnexpectedChar {
        /// Byte offset of the character.
        at: usize,
        /// The character found.
        found: char,
    },
    /// The input ended in the middle of an encoding.
    UnexpectedEnd,
    /// A well-formed spelling that the encoder never produces, such as a leading zero or a
    /// redundant length prefix. Accepting it would give one number two encodings, which breaks
    /// equality and range scans over encoded keys.
    NonCanonical {
        /// Byte offset of the offending group.
        at: usize,
    },
    /// A valid encoding of a number that does not fit the requested type.
    OutOfRange,
    /// A complete encoding followed by more input, where the whole input was expected to be one
    /// encoding. Use a `decode_prefix` function to read an encoding from the front of a longer
    /// string.
    TrailingInput {
        /// Byte offset where the encoding ended.
        at: usize,
    },
    /// A NaN, which has no place in numeric order.
    NotFinite,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => f.write_str("empty input"),
            Error::UnexpectedChar { at, found } => {
                write!(f, "unexpected character {found:?} at byte {at}")
            }
            Error::UnexpectedEnd => f.write_str("input ended inside an encoding"),
            Error::NonCanonical { at } => write!(f, "non-canonical encoding at byte {at}"),
            Error::OutOfRange => f.write_str("number out of range for the target type"),
            Error::TrailingInput { at } => write!(f, "trailing input at byte {at}"),
            Error::NotFinite => f.write_str("NaN cannot be encoded"),
        }
    }
}

impl core::error::Error for Error {}
