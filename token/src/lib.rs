#![feature(try_from)]

use failure_derive::Fail;
use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};

use std::convert::TryFrom;
use std::fmt::{self, Debug, Display, Formatter};
use std::str;

#[derive(PartialEq, Eq, Hash, Copy, Clone, PartialOrd, Ord)]
pub struct Token(u64);

#[derive(Debug, Fail)]
pub enum TokenError {
    #[fail(display = "symbol may not exceed 8 bytes, got {}", _0)]
    TokenTooLong(usize),
}

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        let n = self.0;
        let bytes = [
            (n >> 56) as u8,
            (n >> 48) as u8,
            (n >> 40) as u8,
            (n >> 32) as u8,
            (n >> 24) as u8,
            (n >> 16) as u8,
            (n >> 8) as u8,
            n as u8,
        ];
        write!(
            f,
            "Token({:?})",
            str::from_utf8(&bytes)
                .map_err(|_| fmt::Error)?
                .trim_right_matches('\0')
        )
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        let n = self.0;
        let bytes = [
            (n >> 56) as u8,
            (n >> 48) as u8,
            (n >> 40) as u8,
            (n >> 32) as u8,
            (n >> 24) as u8,
            (n >> 16) as u8,
            (n >> 8) as u8,
            n as u8,
        ];
        write!(
            f,
            "{}",
            str::from_utf8(&bytes)
                .map_err(|_| fmt::Error)?
                .trim_right_matches('\0')
        )
    }
}

impl Serialize for Token {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        let n = self.0;
        let bytes = [
            (n >> 56) as u8,
            (n >> 48) as u8,
            (n >> 40) as u8,
            (n >> 32) as u8,
            (n >> 24) as u8,
            (n >> 16) as u8,
            (n >> 8) as u8,
            n as u8,
        ];
        let s = str::from_utf8(&bytes)
            .map_err(S::Error::custom)?
            .trim_right_matches('\0');
        serializer.serialize_str(s)
    }
}

impl TryFrom<&str> for Token {
    type Error = TokenError;

    fn try_from(s: &str) -> Result<Self, TokenError> {
        let bytes = s.bytes();
        let shift = 8 * (8usize
            .checked_sub(bytes.len())
            .ok_or_else(|| TokenError::TokenTooLong(bytes.len()))?);
        Ok(Token(bytes.fold(0, |n, x| (n << 8) + x as u64) << shift))
    }
}

impl Deserialize<'de> for Token {
    fn deserialize<D>(deserializer: D) -> Result<Token, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TokenVisitor;
        impl Visitor<'_> for TokenVisitor {
            type Value = Token;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a currency symbol")
            }

            fn visit_str<E>(self, value: &str) -> Result<Token, E>
            where
                E: de::Error,
            {
                Token::try_from(value).map_err(E::custom)
            }
        }
        deserializer.deserialize_str(TokenVisitor)
    }
}

impl From<Token> for Box<str> {
    fn from(t: Token) -> Box<str> {
        let n = t.0;
        let mut bytes = vec![
            (n >> 56) as u8,
            (n >> 48) as u8,
            (n >> 40) as u8,
            (n >> 32) as u8,
            (n >> 24) as u8,
            (n >> 16) as u8,
            (n >> 8) as u8,
            n as u8,
        ];
        if let Some((len, _)) = bytes.iter().cloned().enumerate().find(|(_, x)| *x == 0) {
            bytes.truncate(len);
        }
        let bytes = bytes.into_boxed_slice();
        unsafe { str::from_boxed_utf8_unchecked(bytes) }
    }
}
