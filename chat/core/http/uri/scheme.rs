//! URI scheme
//! IMplementation refers to RFC 3986 -> https://datatracker.ietf.org/doc/html/rfc3986
//!
//!
//! https://datatracker.ietf.org/doc/html/rfc3986#section-3.1
//!
//!
use crate::core::http::{
    chars_sets::SCHEME,
    error::ErrorKind::{self, InvalidScheme, InvalidSchemeLength},
};

// [dev]:
// maybe change position of max_length check
// make MAX_SCHEME_LEN configurable & max_length not presented in specs so it's custom
const MAX_SCHEME_LEN: usize = 64;

#[derive(Debug)]
pub enum Scheme {
    None,
    Standard(Protocol),
    Other(String),
}

#[derive(Debug)]
pub enum Protocol {
    Http,
    Https,
}

impl Scheme {
    // [ dev ]:
    // maybe it's better to check uri represented with bytes and not a str?!
    // for example conversion it with str.as_bytes() in fn parse arguments
    /// Parses a URI scheme string and validates its correctness.
    pub fn parse(uri_str: &str) -> Result<Scheme, ErrorKind> {
        let uri_len = uri_str.len();

        if uri_len > MAX_SCHEME_LEN {
            return Err(InvalidSchemeLength(uri_len));
        }

        if !uri_str.chars().all(|ch| SCHEME.contains(&ch)) {
            return Err(ErrorKind::InvalidScheme(uri_str.to_string()));
        }

        match uri_str {
            "http" => Ok(Scheme::Standard(Protocol::Http)),
            "https" => Ok(Scheme::Standard(Protocol::Https)),
            _ => Ok(Scheme::Other(uri_str.to_string())),
        }
    }
}

impl TryFrom<&[u8]> for Scheme {
    type Error = ErrorKind;
    fn try_from(uri_bytes: &[u8]) -> Result<Self, Self::Error> {
        Scheme::parse(std::str::from_utf8(uri_bytes).unwrap())
    }
}
