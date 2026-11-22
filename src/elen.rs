use alloc::string::String;
use core::{borrow::Borrow, fmt, str::FromStr};

use crate::{Error, Integer, int};

/// A validated ELEN encoding of an integer, of any size.
///
/// `Elen`s compare in numeric order, since their bytes do.
///
/// ```
/// use elen::Elen;
///
/// let a = Elen::from(-3i64);
/// let b: Elen = ";;212".parse()?;
/// assert!(a < b);
/// assert_eq!(b.decode::<u8>()?, 12);
/// assert_eq!(a.decode::<u8>(), Err(elen::Error::OutOfRange));
/// # Ok::<(), elen::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Elen(String);

impl Elen {
    /// Encodes an integer.
    pub fn new<T: Integer>(value: &T) -> Self {
        Self(crate::encode(value))
    }

    /// Decodes the integer, failing with [`Error::OutOfRange`] when it does not fit `T`.
    pub fn decode<T: Integer>(&self) -> Result<T, Error> {
        T::decode(&self.0)
    }

    /// The encoding.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The encoding.
    pub fn into_string(self) -> String {
        self.0
    }

    fn validate(s: &str) -> Result<(), Error> {
        let token = int::read(s, usize::MAX, int::depth(usize::MAX))?;
        int::finish(s, ((), &s[token.end..]))
    }
}

impl FromStr for Elen {
    type Err = Error;

    /// Validates that `s` is exactly the encoding of some integer.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::validate(s)?;
        Ok(Self(s.into()))
    }
}

impl TryFrom<&str> for Elen {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for Elen {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::validate(&value)?;
        Ok(Self(value))
    }
}

impl fmt::Display for Elen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Elen {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Elen {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<Elen> for String {
    fn from(value: Elen) -> Self {
        value.0
    }
}

macro_rules! conversions {
    ($($t:ty),*) => {$(
        impl From<$t> for Elen {
            fn from(value: $t) -> Self {
                Elen::new(&value)
            }
        }

        impl TryFrom<&Elen> for $t {
            type Error = Error;

            fn try_from(value: &Elen) -> Result<Self, Self::Error> {
                value.decode()
            }
        }

        impl TryFrom<Elen> for $t {
            type Error = Error;

            fn try_from(value: Elen) -> Result<Self, Self::Error> {
                value.decode()
            }
        }
    )*};
}

conversions!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

#[cfg(feature = "serde")]
mod serde_impls {
    use alloc::string::String;

    use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

    use super::Elen;

    impl Serialize for Elen {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(&self.0)
        }
    }

    impl<'de> Deserialize<'de> for Elen {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            Elen::try_from(String::deserialize(deserializer)?).map_err(de::Error::custom)
        }
    }
}
